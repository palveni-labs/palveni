use super::*;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Reply {
    code: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}
impl Reply {
    fn new(code: u16, mime: &str, body: &str) -> Self {
        Self {
            code,
            headers: vec![("Content-Type", mime.into())],
            body: body.into(),
        }
    }
    fn json(body: &str) -> Self {
        Self::new(200, "application/json", body)
    }
    fn header(mut self, key: &'static str, v: &str) -> Self {
        self.headers.push((key, v.to_owned()));
        self
    }
}
struct Server {
    base: Url,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start(handle: impl Fn(&str, &str, &str) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = Url::parse(&format!("http://{}/media", listener.local_addr().unwrap())).unwrap();
        let handle = Arc::new(handle);
        let task = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let handle = handle.clone();
                tokio::spawn(async move {
                    let mut data = Vec::new();
                    let mut buf = [0; 1024];
                    while !data.windows(4).any(|w| w == b"\r\n\r\n") && data.len() < 16 * 1024 {
                        let Ok(n) = socket.read(&mut buf).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        data.extend_from_slice(&buf[..n]);
                    }
                    let request = String::from_utf8_lossy(&data);
                    let mut first = request.lines().next().unwrap().split_whitespace();
                    let reply = handle(
                        first.next().unwrap(),
                        first.next().unwrap(),
                        &request.to_ascii_lowercase(),
                    );
                    let mut head = format!(
                        "HTTP/1.1 {} Test\r\nConnection: close\r\nContent-Length: {}\r\n",
                        reply.code,
                        reply.body.len()
                    );
                    for (k, v) in reply.headers {
                        head.push_str(&format!("{k}: {v}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(reply.body.as_bytes()).await;
                });
            }
        });
        Self { base, task }
    }
}
fn secret() -> Credentials {
    Credentials {
        username: "a".into(),
        secret: "b".into(),
    }
}
fn media_reply(headers: &str) -> Reply {
    if headers.contains("range: bytes=2-4") {
        Reply::new(206, "video/mp4", "cde").header("Content-Range", "bytes 2-4/6")
    } else if headers.contains("range: bytes=99-") {
        Reply::new(416, "text/plain", "").header("Content-Range", "bytes */6")
    } else {
        Reply::new(200, "audio/mpeg", "abcdef")
    }
}
async fn content(mut stream: RemoteStream) -> String {
    let mut body = String::new();
    stream.reader.read_to_string(&mut body).await.unwrap();
    body
}

#[tokio::test]
async fn webdav_scopes_collections_and_streams_authenticated_ranges() {
    let dav = Server::start(|method, path, headers| {
        if !headers.contains("authorization: basic ytpi") {
            return Reply::new(401, "text/plain", "");
        }
        if method == "PROPFIND" && path == "/media/" && headers.contains("depth: 1") {
            let rows = [
                ("/media/", "<D:collection/>", "Self"),
                ("/media/Shows/", "<D:collection/>", "Cinema"),
                ("/media/Lake%20Day.mp4", "", "Lake &amp; Day"),
                ("/media/Shows/extra.mp4", "", "Grandchild"),
                ("/other/hidden.mp4", "", "Outside"),
            ]
            .map(|(path, ty, name)| {
                format!(
                    "<D:response><D:href>{path}</D:href><D:propstat><D:prop>
                 <D:resourcetype>{ty}</D:resourcetype><D:displayname>{name}</D:displayname>
                 </D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>"
                )
            })
            .join("");
            return Reply::new(
                207,
                "application/xml",
                &format!("<D:multistatus xmlns:D='DAV:'>{rows}</D:multistatus>"),
            );
        }
        if method == "GET" && path == "/media/Lake%20Day.mp4" {
            return media_reply(headers);
        }
        Reply::new(404, "text/plain", "")
    })
    .await;
    let kind = ConnectionKind::Webdav;
    let creds = secret();
    let first = entries(kind, &dav.base, &creds, "", 0, 1).await.unwrap();
    assert_eq!(first.entries[0].id, "folder:Shows");
    assert_eq!(first.next_offset, Some(1));
    let second = entries(kind, &dav.base, &creds, "", 1, 5).await.unwrap();
    assert_eq!(second.entries.len(), 1);
    assert_eq!(second.entries[0].name, "Lake & Day");
    let id = &second.entries[0].id;
    let stream = open(kind, &dav.base, &creds, id, Some("bytes=2-4"))
        .await
        .unwrap();
    assert_eq!(
        (stream.partial, stream.len, stream.start, stream.total),
        (true, 3, 2, 6)
    );
    assert_eq!(content(stream).await, "cde");
    assert!(matches!(
        open(kind, &dav.base, &creds, id, Some("bytes=99-")).await,
        Err(RemoteError::Range { total: 6 })
    ));
    assert!(matches!(
        open(kind, &dav.base, &creds, "video:../archive.mp4", None).await,
        Err(RemoteError::Invalid)
    ));
}

#[tokio::test]
async fn jellyfin_and_emby_discover_token_libraries_page_items_and_read_originals() {
    for kind in [ConnectionKind::Jellyfin, ConnectionKind::Emby] {
        let srv = Server::start(move |method, path, headers| {
            if !matches!(method, "GET" | "HEAD")
                || !headers.contains("x-emby-token: b")
                || !headers.contains("authorization: mediabrowser")
            {
                return Reply::new(401, "text/plain", "");
            }
            if kind == ConnectionKind::Emby && path == "/media/Users/Me" {
                return Reply::new(500, "text/plain", "Invalid user id");
            }
            if (kind == ConnectionKind::Jellyfin && path == "/media/UserViews")
                || (kind == ConnectionKind::Emby && path == "/media/Library/MediaFolders")
            {
                return Reply::json(
                    r#"{"Items":[{"Id":"films","Name":"Film library","Type":"CollectionFolder"}]}"#,
                );
            }
            if path.starts_with("/media/Items?")
                && path.contains("ParentId=films")
                && path.contains("StartIndex=1")
                && path.contains("Limit=2")
            {
                return Reply::json(
                    r#"{"Items":[
                    {"Id":"42","Name":"River","Type":"Movie","MediaType":"Video"},
                    {"Id":"50","Name":"Wind","MediaType":"Audio"}],"TotalRecordCount":3}"#,
                );
            }
            if path == "/media/Videos/42/stream?Static=true"
                || path == "/media/Audio/50/stream?Static=true"
            {
                if kind == ConnectionKind::Emby && headers.contains("range: bytes=99-") {
                    return Reply::new(416, "text/plain", "Invalid byte range");
                }
                return media_reply(headers);
            }
            Reply::new(404, "text/plain", "")
        })
        .await;
        let c = secret();
        let root = entries(kind, &srv.base, &c, "", 0, 100).await.unwrap();
        let sub = entries(kind, &srv.base, &c, &root.entries[0].id, 1, 1)
            .await
            .unwrap();
        assert_eq!(sub.next_offset, Some(2));
        assert_eq!(sub.entries.len(), 1);
        assert_eq!(
            content(
                open(kind, &srv.base, &c, &sub.entries[0].id, Some("bytes=2-4"))
                    .await
                    .unwrap()
            )
            .await,
            "cde"
        );
        assert_eq!(
            content(open(kind, &srv.base, &c, "audio:50", None).await.unwrap()).await,
            "abcdef"
        );
        assert!(matches!(
            open(kind, &srv.base, &c, "video:42", Some("bytes=99-")).await,
            Err(RemoteError::Range { total: 6 })
        ));
    }
}

#[tokio::test]
async fn plex_uses_catalog_rating_keys_and_the_selected_original_part() {
    let srv = Server::start(|_, path, headers| {
        if !headers.contains("x-plex-token: b")
            || (!path.contains("/library/parts/") && !headers.contains("accept: application/json"))
        {
            return Reply::new(401, "text/plain", "");
        }
        match path.split('?').next().unwrap() {
            "/media/library/sections" => {
                Reply::json(r#"{"MediaContainer":{"Directory":[{"key":"1","title":"Movies"}]}}"#)
            }
            "/media/library/sections/1/all" if path.contains("X-Plex-Container-Start=0") => {
                Reply::json(
                    r#"{"MediaContainer":{"offset":0,"size":1,"totalSize":1,
                "Metadata":[{"ratingKey":"2","title":"Seasons","type":"show"}]}}"#,
                )
            }
            "/media/library/metadata/2/children" => Reply::json(
                r#"{"MediaContainer":{"Metadata":[{"ratingKey":"3","title":"River","type":"episode",
                "Media":[{"Part":[{"id":7},{"id":8}]}]}]}}"#,
            ),
            "/media/library/metadata/3" => Reply::json(
                r#"{"MediaContainer":{"Metadata":[{"ratingKey":"3","title":"River","type":"episode",
                "Media":[{"Part":[{"id":7,"key":"/library/parts/7/123/file.mp4"},
                {"id":8,"key":"/library/parts/8/124/file.mp4"}]}]}]}}"#,
            ),
            "/media/library/parts/8/124/file.mp4" => media_reply(headers),
            _ => Reply::new(404, "text/plain", ""),
        }
    })
    .await;
    let c = secret();
    let k = ConnectionKind::Plex;
    let libraries = entries(k, &srv.base, &c, "", 0, 50).await.unwrap();
    let series = entries(k, &srv.base, &c, &libraries.entries[0].id, 0, 50)
        .await
        .unwrap();
    let recordings = entries(k, &srv.base, &c, &series.entries[0].id, 0, 50)
        .await
        .unwrap();
    let parts = entries(k, &srv.base, &c, &recordings.entries[0].id, 0, 50)
        .await
        .unwrap();
    assert_eq!(parts.entries.len(), 2);
    assert!(parts.entries[1].kind == Kind::Video);
    assert_eq!(
        content(
            open(k, &srv.base, &c, &parts.entries[1].id, Some("bytes=2-4"))
                .await
                .unwrap()
        )
        .await,
        "cde"
    );
}

#[tokio::test]
async fn redirects_never_forward_credentials_to_another_origin() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    let other = Server::start(move |_, _, _| {
        seen.fetch_add(1, Ordering::Relaxed);
        Reply::json("{}")
    })
    .await;
    let location = other.base.to_string();
    let first = Server::start(move |_, _, _| {
        Reply::new(302, "text/plain", "").header("Location", &location)
    })
    .await;
    assert!(matches!(
        entries(ConnectionKind::Plex, &first.base, &secret(), "", 0, 1).await,
        Err(RemoteError::Unavailable)
    ));
    assert_eq!(count.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn webdav_requires_readable_collection_and_rejects_non_media_responses() {
    let dav = Server::start(|method, _, _| {
        if method == "PROPFIND" {
            Reply::new(
                207,
                "application/xml",
                "<multistatus xmlns='DAV:'><response><href>/media/</href>
                <status>HTTP/1.1 403 Forbidden</status></response></multistatus>",
            )
        } else {
            Reply::new(200, "text/html", "sign-in page")
        }
    })
    .await;
    let k = ConnectionKind::Webdav;
    assert!(matches!(
        entries(k, &dav.base, &secret(), "", 0, 5).await,
        Err(RemoteError::Unavailable)
    ));
    assert!(matches!(
        open(k, &dav.base, &secret(), "video:movie.mp4", None).await,
        Err(RemoteError::Unavailable)
    ));
}
