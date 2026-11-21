//! Album images are read on demand, independently from the scan.
use crate::LocalStorage;
use lofty::{config::ParseOptions, file::TaggedFileExt, picture::PictureType, probe::Probe};
use palveni_core::{Error, ItemId, Result, storage::StoragePath};
use palveni_db::{Db, music::TrackLocation};
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use tokio::sync::Semaphore;

const MAX_IMAGE: usize = 8 * 1024 * 1024;
const MAX_TAG_READ: usize = MAX_IMAGE * 2;
static READERS: Semaphore = Semaphore::const_new(2);

pub(crate) fn mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}

// Seek past audio payloads freely while bounding actual reads across all tag blocks.
struct TagReader {
    file: std::fs::File,
    remaining: usize,
}
impl Read for TagReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 && !buf.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "metadata read limit reached",
            ));
        }
        let len = buf.len().min(self.remaining);
        let n = self.file.read(&mut buf[..len])?;
        self.remaining -= n;
        Ok(n)
    }
}
impl Seek for TagReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

pub(crate) fn bounded_tag_reader(file: std::fs::File) -> impl Read + Seek {
    BufReader::new(TagReader {
        file,
        remaining: MAX_TAG_READ,
    })
}

const NAMES: [&str; 12] = [
    "cover.jpg",
    "cover.jpeg",
    "cover.png",
    "cover.webp",
    "folder.jpg",
    "folder.jpeg",
    "folder.png",
    "folder.webp",
    "front.jpg",
    "front.jpeg",
    "front.png",
    "front.webp",
];

fn sidecar_names(root: &LocalStorage, parent: &StoragePath) -> Vec<String> {
    let mut names = Vec::new();
    let mut probe_standard = true;
    if let Ok(entries) = root.directory_entries(parent) {
        probe_standard = false;
        for (i, entry) in entries.enumerate() {
            if i == 256 {
                probe_standard = true;
                break;
            }
            let Some(name) = entry
                .ok()
                .and_then(|entry| entry.file_name().into_string().ok())
            else {
                continue;
            };
            if let Some(rank) = NAMES
                .iter()
                .position(|candidate| candidate.eq_ignore_ascii_case(&name))
            {
                names.push((rank, name));
            }
        }
    }
    if probe_standard {
        names.extend(
            NAMES
                .iter()
                .enumerate()
                .map(|(rank, name)| (rank, (*name).to_owned())),
        );
    }
    names.sort_unstable();
    names.dedup();
    names.into_iter().map(|(_, name)| name).collect()
}

fn read_image(mut file: std::fs::File) -> Option<(&'static str, Vec<u8>)> {
    let len = file.metadata().ok()?.len();
    if !(12..=MAX_IMAGE as u64).contains(&len) {
        return None;
    }
    let mut signature = [0; 12];
    file.read_exact(&mut signature).ok()?;
    let media = mime(&signature)?;
    let mut data = Vec::with_capacity(usize::try_from(len).ok()?);
    data.extend_from_slice(&signature);
    file.take((MAX_IMAGE + 1 - signature.len()) as u64)
        .read_to_end(&mut data)
        .ok()?;
    (data.len() <= MAX_IMAGE).then_some((media, data))
}

type Image = (&'static str, Vec<u8>);

pub async fn open(db: &Db, id: ItemId) -> Result<Image> {
    let music = db.music();
    let location = music
        .track_location(id)
        .await?
        .ok_or_else(|| Error::not_found("track"))?;
    if let Some(image) = from_track(location).await? {
        return Ok(image);
    }
    for sibling in music.artwork_siblings(id).await? {
        if let Some(location) = music.track_location(sibling).await?
            && let Ok(Some(image)) = from_track(location).await
        {
            return Ok(image);
        }
    }
    Err(Error::not_found("artwork"))
}

async fn from_track(location: TrackLocation) -> Result<Option<Image>> {
    let permit = READERS
        .acquire()
        .await
        .map_err(|e| Error::internal(e.to_string()))?;
    let (root, track) = crate::local::track_path(location).await?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let audio = root.open_file_blocking(&track)?;
        let parent = track.parent().unwrap_or_else(StoragePath::root);
        for name in sidecar_names(&root, &parent) {
            let cover = parent.join(&name)?;
            if let Ok(file) = root.open_file_blocking(&cover)
                && let Some(image) = read_image(file)
            {
                return Ok(Some(image));
            }
        }
        crate::tags::limit_allocations();
        let reader = bounded_tag_reader(audio);
        let options = ParseOptions::new()
            .read_cover_art(true)
            .read_properties(false);
        let Ok(tags) = Probe::new(reader)
            .options(options)
            .guess_file_type()?
            .read()
        else {
            return Ok(None);
        };
        let pictures = || tags.tags().iter().flat_map(|tag| tag.pictures());
        Ok(pictures()
            .filter(|p| p.pic_type() == PictureType::CoverFront)
            .chain(pictures())
            .filter(|p| p.data().len() <= MAX_IMAGE)
            .find_map(|pic| mime(pic.data()).map(|media| (media, pic.data().to_vec()))))
    })
    .await
    .map_err(|e| Error::internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn skips_oversized_and_unrecognized_files_and_rechecks_the_track() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let track = root.join("record.flac");
        fs::write(&track, b"untagged").unwrap();
        fs::File::create(root.join("cover.jpg"))
            .unwrap()
            .set_len(MAX_IMAGE as u64 + 1)
            .unwrap();
        fs::write(root.join("cover.png"), b"unsupported data").unwrap();
        let cover = b"RIFF\x10\x00\x00\x00WEBPfixture";
        fs::write(root.join("fOLDer.WeBp"), cover).unwrap();
        let location = || TrackLocation {
            cue_frames: None,
            root: root.clone(),
            relative_path: "record.flac".into(),
        };
        assert_eq!(
            from_track(location()).await.unwrap().unwrap(),
            ("image/webp", cover.to_vec())
        );
        fs::remove_file(track).unwrap();
        assert!(from_track(location()).await.is_err());
    }

    #[tokio::test]
    async fn reads_the_front_picture_even_when_the_back_is_first_in_flac() {
        use lofty::picture::{MimeType, Picture, PictureInformation};

        let mut flac = b"fLaC\x00\x00\x00\x22".to_vec();
        // One 400-sample mono stream. The artwork reader skips audio frames.
        flac.extend([
            2, 64, 2, 64, 0, 0, 13, 0, 0, 13, 1, 244, 0, 240, 0, 0, 1, 144,
        ]);
        flac.extend([0; 16]);
        let front: &[u8] = b"\x89PNG\r\n\x1a\nfront";
        let back: &[u8] = b"\x89PNG\r\n\x1a\nback";
        for (header, kind, image) in [
            (6, PictureType::CoverBack, back),
            (0x86, PictureType::CoverFront, front),
        ] {
            let picture = Picture::unchecked(image.to_vec())
                .mime_type(MimeType::Png)
                .pic_type(kind)
                .build();
            let block = picture.as_flac_bytes(PictureInformation::default(), false);
            let length = u32::try_from(block.len()).unwrap().to_be_bytes();
            flac.push(header);
            flac.extend(&length[1..]);
            flac.extend(block);
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::write(root.join("song.flac"), flac).unwrap();
        let location = TrackLocation {
            cue_frames: None,
            root,
            relative_path: "song.flac".into(),
        };
        assert_eq!(
            from_track(location).await.unwrap().unwrap(),
            ("image/png", front.to_vec())
        );
    }

    #[test]
    fn seeking_does_not_reset_the_embedded_tag_read_limit() {
        let mut file = tempfile::tempfile().unwrap();
        file.set_len(100).unwrap();
        file.seek(SeekFrom::Start(25)).unwrap();
        let mut reader = TagReader { file, remaining: 5 };
        let mut buf = [0; 16];
        assert_eq!(reader.read(&mut buf).unwrap(), 5);
        reader.seek(SeekFrom::Start(0)).unwrap();
        assert!(reader.read(&mut buf).is_err());
        assert_eq!(reader.read(&mut []).unwrap(), 0);
    }
}
