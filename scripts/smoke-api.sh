#!/usr/bin/env bash
# End-to-end check of the HTTP API against a running Palveni server.
#
# The server must be FRESH (no user created yet) and must run on the same
# machine as this script, because the script creates a temporary media folder
# and asks the server to scan it.
#
#   PALVENI_DATA_DIR=$(mktemp -d) PALVENI_PORT=7258 ./target/release/palveni serve &
#   scripts/smoke-api.sh                       # defaults to http://127.0.0.1:7258
#   scripts/smoke-api.sh http://127.0.0.1:7300
# For Docker, bind-mount PALVENI_SMOKE_MEDIA_DIR read-only at the same
# absolute path. Only the generated media fixture is shared.
#
# Needs: curl, jq, python3. Exits non-zero if any check fails.

set -u

BASE="${1:-http://127.0.0.1:7258}"
API="$BASE/api/v1"
USERNAME="smoke-admin"
PASSWORD="smoke-test-$(date +%s)"
PRIVATE_TMP="$(mktemp -d)" || exit 2
MEDIA="$(mktemp -d "${PALVENI_SMOKE_MEDIA_DIR:-${TMPDIR:-/tmp}}/palveni-media.XXXXXXXX")" || { rmdir "$PRIVATE_TMP"; exit 2; }
GONE="$MEDIA.gone"
PASS=0
FAIL=0
TOKEN=""

cleanup() { rm -rf "$MEDIA" "$GONE" "$PRIVATE_TMP"; }
trap cleanup EXIT
if [ -n "${PALVENI_SMOKE_MEDIA_DIR:-}" ]; then
  # The packaged server runs as a separate, unprivileged UID.
  chmod 755 "$MEDIA" || exit 2
  umask 022
fi

ok()   { PASS=$((PASS + 1)); printf '  ok    %s\n' "$1"; }
bad()  { FAIL=$((FAIL + 1)); printf '  FAIL  %s\n        %s\n' "$1" "$2"; }

# check <description> <actual> <expected>
check() {
  if [ "$2" = "$3" ]; then ok "$1"; else bad "$1" "expected: $3   got: $2"; fi
}

# req <method> <path> [json-body]  ->  sets STATUS and BODY
req() {
  local method="$1" path="$2" body="${3:-}" out
  local args=(-sS -D "$PRIVATE_TMP/headers" -o "$PRIVATE_TMP/body" -w '%{http_code}' -X "$method")
  [ -n "$TOKEN" ] && args+=(-H "authorization: Bearer $TOKEN")
  [ -n "$body" ] && args+=(-H 'content-type: application/json' -d "$body")
  out="$(curl "${args[@]}" "$API$path" 2>/dev/null)" || out="000"
  STATUS="$out"
  BODY="$(cat "$PRIVATE_TMP/body" 2>/dev/null)"
  rm -f "$PRIVATE_TMP/body"
}

field() { printf '%s' "$BODY" | jq -r "$1" 2>/dev/null; }

# Keep binary responses out of shell variables.
read_media() {
  curl -sS --max-time 10 -D "$PRIVATE_TMP/media.headers" -o "$PRIVATE_TMP/audio" -w '%{http_code}' "$@"
}

media_header() {
  awk -v key="$1:" 'tolower($1) == key {sub(/^[^:]+: */, ""); sub(/\r$/, ""); print; exit}' "$PRIVATE_TMP/media.headers"
}

wait_for_job() {
  local id="$1" state=""
  for _ in $(seq 1 100); do
    req GET "/jobs?limit=20"
    state="$(field ".jobs[] | select(.id == \"$id\") | .state")"
    case "$state" in succeeded|failed|cancelled) break ;; esac
    sleep 0.2
  done
  JOB="$(printf '%s' "$BODY" | jq -c ".jobs[] | select(.id == \"$id\")")"
  JOB_STATE="$state"
}

command -v jq >/dev/null || { echo "jq is required"; exit 2; }

echo "Palveni API smoke test against $BASE"

# ---------------------------------------------------------------- system
echo "System"
req GET /system/health
check "health is ok" "$STATUS $(field .status)" "200 ok"

req GET /system/info
check "info responds" "$STATUS" "200"
if [ "$(field .setupCompleted)" != "false" ]; then
  echo "This server already has a user. Start a fresh one (empty data directory) and run again."
  exit 2
fi
check "info lists features" "$(field '.features | index("libraries") != null')" "true"

req GET /openapi.json
check "public OpenAPI defines settings, login throttling and event cursors" \
  "$STATUS $(field '.openapi == "3.1.0" and (.paths["/api/v1/system/settings"].put != null) and (.paths["/api/v1/auth/login"].post.responses["429"] != null) and any(.paths["/api/v1/events"].get.parameters[]; (.name | ascii_downcase) == "last-event-id")')" "200 true"
req GET /system/capabilities
check "capabilities require login" "$STATUS" "401"
req GET /system/settings
check "settings require login" "$STATUS" "401"

# ---------------------------------------------------------------- setup and login
echo "Setup and login"
req GET /libraries
check "protected endpoint without token is 401" "$STATUS $(field .error.code)" "401 unauthorized"

req POST /setup "{\"username\":\"$USERNAME\",\"password\":\"short\"}"
check "short password is rejected" "$STATUS $(field .error.code)" "400 validation_failed"

req POST /setup "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}"
check "setup creates the admin" "$STATUS $(field .user.role)" "201 admin"

req POST /setup "{\"username\":\"other\",\"password\":\"$PASSWORD\"}"
check "second setup is refused" "$STATUS $(field .error.code)" "409 setup_already_completed"

req POST /auth/login "{\"username\":\"$USERNAME\",\"password\":\"wrong-password\"}"
check "wrong password is refused" "$STATUS $(field .error.code)" "401 invalid_credentials"

req POST /auth/login "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\",\"deviceName\":\"smoke script\"}"
check "login succeeds" "$STATUS $(field .tokenType)" "200 Bearer"
TOKEN="$(field .accessToken)"

req GET /auth/me
check "me returns the admin" "$STATUS $(field .username)" "200 $USERNAME"

echo "Settings, capabilities and resumable events"
req GET /system/capabilities
check "admin can discover runtime settings with method and access level" \
  "$STATUS $(field 'any(.capabilities[]; .path == "/api/v1/system/settings" and .method == "PUT" and .permission == "admin")')" "200 true"
req GET /system/settings
saved_settings="$BODY"
check "fresh server has no runtime overrides" "$STATUS $(field '. == {serverName:null,perfTier:null}')" "200 true"
req PUT /system/settings '{"serverName":"x"}'
check "replacement requires both nullable fields" "$STATUS $(field .error.code)" "400 validation_failed"
req PUT /system/settings '{"serverName":"   ","perfTier":null}'
check "blank server name is rejected with the field" "$STATUS $(field .error.details.field)" "400 serverName"

# Obtain a durable boundary before writing, so the replay assertion also exercises reconnect.
read_events() {
  curl -sN --max-time 1 -H "authorization: Bearer $TOKEN" -H "Last-Event-ID: $1" "$API/events" > "$PRIVATE_TMP/events"
  local rc=$?
  [ "$rc" -eq 0 ] || [ "$rc" -eq 28 ]
}
read_events 9223372036854775807
boundary=$(awk '/^id: / {print $2; exit}' "$PRIVATE_TMP/events")
check "future cursor gets reset with a decimal boundary" "$(awk '/^event: stream.reset$/ {seen=1} END {print seen+0}' "$PRIVATE_TMP/events") $(printf '%s' "$boundary" | jq -Rs 'test("^[0-9]+$")')" "1 true"
req PUT /system/settings '{"serverName":"  Palveni smoke  ","perfTier":"tiny"}'
check "admin can change name and performance tier" "$STATUS $(field '.serverName + "/" + .perfTier')" "200 Palveni smoke/tiny"
req GET /system/info
check "renamed server is visible immediately" "$(field .name)" "Palveni smoke"
req GET /system/hardware
check "runtime tier is applied immediately" "$(field .perfTier)" "tiny"
read_events "$boundary"
check "reconnect delivers the numbered settings change" "$(python3 - "$PRIVATE_TMP/events" "$boundary" <<'PYTHON'
import json, sys
for block in open(sys.argv[1]).read().split("\n\n"):
    fields = dict(line.split(": ", 1) for line in block.splitlines() if ": " in line)
    if fields.get("event") == "settings.updated" and int(fields["id"]) > int(sys.argv[2]):
        if json.loads(fields["data"]) == {"serverName": "Palveni smoke", "perfTier": "tiny"}:
            print("yes")
            break
PYTHON
)" "yes"
check "malformed SSE cursor is rejected" "$(curl -s -o /dev/null -w '%{http_code}' -H "authorization: Bearer $TOKEN" -H 'Last-Event-ID: invalid' "$API/events")" "400"
req PUT /system/settings "$saved_settings"
check "runtime overrides can be cleared" "$STATUS $(field '.serverName == null and .perfTier == null')" "200 true"

# ---------------------------------------------------------------- libraries
echo "Libraries"
mkdir -p "$MEDIA/Album A" "$MEDIA/Album B/Disc 1" "$MEDIA/空文件夹"
for i in 1 2 3; do : > "$MEDIA/Album A/track $i.flac"; : > "$MEDIA/Album B/Disc 1/歌曲 $i.mp3"; done
# A 50 ms mono silent FLAC with Vorbis comments; no media tools are needed on the test runner.
python3 - "$MEDIA/Album A" <<'PY'
import pathlib, struct, sys
root = pathlib.Path(sys.argv[1])
info = bytes.fromhex("0240024000000d00000d01f400f0000001906b431bf2da7c9312b3e5c21e67f6591b")
frame = bytes.fromhex("fff8740800018f240000007d5f")
def field_bytes(data):
    return struct.pack("<I", len(data)) + data
for num, title in enumerate(("First Light", "Interlude", "Coda"), 1):
    fields = [f"TITLE={title}", "ARTIST=Guest Quartet", "ALBUMARTIST=North Ensemble",
              "ALBUM=Early Sessions", "DISCNUMBER=1", f"TRACKNUMBER={num}"]
    if num == 1:
        fields.append("LYRICS=Embedded test line")
    tags = field_bytes(b"Palveni") + struct.pack("<I", len(fields))
    tags += b"".join(field_bytes(value.encode()) for value in fields)
    (root / f"track {num}.flac").write_bytes(
        b"fLaC\0\0\0\x22" + info + b"\x84" + len(tags).to_bytes(3, "big") + tags + frame)
PY
: > "$MEDIA/notes.txt"

req POST /libraries '{"name":"相对路径","kind":"music","paths":["relative/path"]}'
check "relative path is rejected" "$STATUS $(field .error.code)" "400 validation_failed"

req POST /libraries "{\"name\":\"不存在\",\"kind\":\"music\",\"paths\":[\"$MEDIA\",\"$MEDIA/does-not-exist\"]}"
check "missing path is reported with its position" \
  "$STATUS $(field .error.code) $(field .error.details.index)" "400 path_not_found 1"

req POST /libraries "{\"name\":\"冒烟音乐\",\"kind\":\"music\",\"paths\":[\"$MEDIA\"]}"
check "library is created" "$STATUS $(field .kind) $(field .itemCount)" "201 music 0"
LIB="$(field .id)"

req POST /libraries "{\"name\":\"冒烟音乐\",\"kind\":\"music\",\"paths\":[\"$MEDIA\"]}"
check "duplicate name conflicts" "$STATUS $(field .error.code)" "409 conflict"

req PUT "/libraries/$LIB"
check "unsupported method is a JSON 405" "$STATUS $(field .error.code)" "405 method_not_allowed"

# ---------------------------------------------------------------- scan
echo "Scan"
req POST "/libraries/$LIB/scan"
check "scan is accepted" "$STATUS" "202"
JOB_ID="$(field .jobId)"
wait_for_job "$JOB_ID"
check "scan succeeds" "$JOB_STATE" "succeeded"
check "job knows its library" "$(printf '%s' "$JOB" | jq -r '.subject.type + " " + .subject.id')" "library $LIB"
check "job ends on a summary step" "$(printf '%s' "$JOB" | jq -r '.step.code + " added=" + (.step.params.added|tostring)')" "scan.summary added=6"

req GET "/libraries/$LIB"
check "library counts 6 media files" "$(field .itemCount)" "6"

req GET "/items?libraryId=$LIB"
check "top level shows the two folders that contain media" \
  "$(field '[.items[] | .kind + ":" + .title] | join(",")')" "folder:Album A,folder:Album B"
FOLDER="$(field '.items[0].id')"

req GET "/items?libraryId=$LIB&parentId=$FOLDER&limit=2"
check "first page has 2 items and a cursor" "$(field '.items | length') $(field '.nextCursor != null')" "2 true"
CURSOR="$(field .nextCursor)"
req GET "/items?libraryId=$LIB&parentId=$FOLDER&limit=2&cursor=$CURSOR"
check "second page has the remaining item and no cursor" "$(field '.items | length') $(field .nextCursor)" "1 null"

req GET "/items?libraryId=$LIB&parentId=00000000-0000-7000-8000-000000000000"
check "unknown parent is 404" "$STATUS $(field .error.code)" "404 not_found"

req GET "/items?libraryId=$LIB&limit=0"
check "limit 0 is rejected" "$STATUS $(field .error.code)" "400 validation_failed"

req GET "/libraries/$LIB/music"
check "tagged album artist and untagged group are browsable" "$STATUS $(field '[.entries[].name] | index("North Ensemble") != null and index(null) != null')" "200 true"
ARTIST=$(field '.entries[] | select(.name=="North Ensemble") | .id')
req GET "/libraries/$LIB/music?artist=$ARTIST"
check "album groups use the album artist" "$STATUS $(field '.entries[0].name')" "200 Early Sessions"
ALBUM=$(field '.entries[0].id')
req GET "/libraries/$LIB/music?artist=$ARTIST&album=$ALBUM&limit=2"
check "tracks expose performer, disc, track and audio duration" "$(field '.entries[0] | [.kind,.title,.artist,.discNumber,.trackNumber,.durationMs] | join("|")')" "track|First Light|Guest Quartet|1|1|50"
MUSIC_CURSOR=$(field .nextCursor)
TRACK=$(field '.entries[0].id')
OTHER_TRACK=$(field '.entries[1].id')
req GET "/libraries/$LIB/music?artist=$ARTIST&album=$ALBUM&limit=2&cursor=$MUSIC_CURSOR"
check "music pagination preserves track order" "$STATUS $(field '.entries[0].title') $(field .nextCursor)" "200 Coda null"
req GET "/libraries/$LIB/music?cursor=$MUSIC_CURSOR"
check "music cursor belongs to its original group" "$STATUS $(field .error.details.field)" "400 cursor"
req GET "/libraries/$LIB/music?album=$ALBUM"
check "albums require their artist" "$STATUS $(field .error.details.field)" "400 artist"

echo "Track lyrics"
req GET "/items/$TRACK/lyrics"
check "embedded lyrics are available" "$STATUS $(field '.lyrics | [.source,.format,.text] | join("|")')" "200 embedded|plain|Embedded test line"
req GET "/items/$OTHER_TRACK/lyrics"
check "missing lyrics are an empty result" "$STATUS $(field .lyrics)" "200 null"
printf '[00:01.20][00:02.30]同步测试\n' > "$MEDIA/Album A/track 1.lrc"
req GET "/items/$TRACK/lyrics"
check "a timed sidecar takes precedence and retains both timestamps" "$STATUS $(field '.lyrics | [.source,.format,(.text | rtrimstr("\n"))] | join("|")')" "200 sidecar|lrc|[00:01.20][00:02.30]同步测试"
printf 'Revised test line' > "$MEDIA/Album A/track 1.lrc"
req GET "/items/$TRACK/lyrics"
check "editing lyrics is visible on the next request" "$(field '.lyrics | [.format,.text] | join("|")')" "plain|Revised test line"
rm "$MEDIA/Album A/track 1.lrc"
req GET "/items/$TRACK/lyrics"
check "removing a sidecar reveals embedded lyrics immediately" "$(field .lyrics.source)" "embedded"
req GET "/items/$FOLDER/lyrics"
check "folders have no track lyrics endpoint" "$STATUS" "404"
check "lyrics require a login" "$(curl -sS -o /dev/null -w '%{http_code}' "$API/items/$TRACK/lyrics")" "401"

echo "Personal media and playlists"
req GET "/me/items/$TRACK"
check "fresh account has no personal state" "$STATUS $(field '[.favorite,.stars,.positionMs,.playCount,.playbackVersion] | join("|")')" "200 false|0|0|0|0"
req PUT "/me/items/$TRACK/favorite" '{"favorite":true}'
check "a track can be favorited" "$STATUS $(field .favorite)" "200 true"
req PUT "/me/items/$TRACK/rating" '{"stars":5}'
check "rating is saved without removing the favorite" "$STATUS $(field '[.stars,.favorite] | join("|")')" "200 5|true"
req PUT "/me/items/$TRACK/rating" '{"stars":6}'
check "six stars is rejected" "$STATUS" "400"
req GET "/me/items?view=favorites"
check "favorites include the selected track and its tagged title" "$STATUS $(field '.items[0].itemId') $(field '.items[0].title')" "200 $TRACK First Light"
req PUT "/me/items/$TRACK/progress" '{"expectedVersion":0,"positionMs":20,"completed":false}'
VERSION=$(field .playbackVersion)
check "playback saves the unfinished position" "$STATUS $(field .positionMs)" "200 20"
req PUT "/me/items/$TRACK/progress" '{"expectedVersion":0,"positionMs":8,"completed":false}'
check "a stale device cannot rewind newer progress" "$STATUS $(field .error.code)" "409 conflict"
req GET "/me/items?view=continue"
check "continue playback returns the saved position" "$STATUS $(field '.items[0].state.positionMs')" "200 20"
DONE=$(jq -nc --argjson version "$VERSION" '{expectedVersion:$version,positionMs:50,completed:true}')
req PUT "/me/items/$TRACK/progress" "$DONE"
req PUT "/me/items/$TRACK/progress" "$DONE"
check "retrying completion counts the play only once" "$STATUS $(field '[.playCount,.positionMs] | join("|")')" "200 1|0"
req GET "/me/items?view=continue"
check "completed track leaves continue playback" "$STATUS $(field '.items | length')" "200 0"
req GET "/me/items?view=recent"
check "completed track remains in recent playback" "$STATUS $(field '.items[0].itemId')" "200 $TRACK"
PLAYLIST=$(python3 -c 'import uuid; print(uuid.uuid4())')
PLAYLIST_PATH="/me/playlists/$PLAYLIST"
MIX=$(jq -nc --arg a "$TRACK" --arg b "$OTHER_TRACK" '{expectedVersion:0,name:"Evening mix",trackIds:[$a,$b,$a]}')
req PUT "$PLAYLIST_PATH" "$MIX"
VERSION=$(field .version)
check "playlists keep ordered duplicate tracks" "$STATUS $(field '.trackIds | join("|")')" "200 $TRACK|$OTHER_TRACK|$TRACK"
req GET "$PLAYLIST_PATH/tracks?version=$VERSION&limit=2"
AFTER=$(field .nextAfter)
check "playlist pages display tagged titles and performers" "$STATUS $(field '.tracks[0] | [.title,.artist,.durationMs] | join("|")') $AFTER" "200 First Light|Guest Quartet|50 1"
req GET "$PLAYLIST_PATH/tracks?version=$VERSION&after=$AFTER&limit=2"
check "the last repeated track keeps its own position" "$STATUS $(field '.tracks[0].position') $(field '.tracks[0].itemId') $(field .nextAfter)" "200 2 $TRACK null"
CHANGE=$(jq -nc --arg a "$OTHER_TRACK" --argjson v "$VERSION" '{expectedVersion:$v,name:"Morning mix",trackIds:[$a]}')
req PUT "$PLAYLIST_PATH" "$CHANGE"
UPDATED=$(field .version)
check "renaming and changing membership are atomic" "$STATUS $(field '[.name,.trackIds[0]] | join("|")')" "200 Morning mix|$OTHER_TRACK"
req GET "$PLAYLIST_PATH/tracks?version=$VERSION"
check "a page from an outdated playlist version returns conflict" "$STATUS" "409"
req PUT "$PLAYLIST_PATH" "$CHANGE"
check "outdated playlist edit returns conflict" "$STATUS" "409"
req GET "/me/playlists?limit=1"
check "saved playlist is visible on the account" "$STATUS $(field '.playlists[0].id')" "200 $PLAYLIST"
req GET "$PLAYLIST_PATH"
check "playlist detail preserves the saved revision" "$STATUS $(field .version)" "200 $UPDATED"
req DELETE "$PLAYLIST_PATH?version=$UPDATED"
check "the owner can delete the playlist" "$STATUS" "204"
req GET "$PLAYLIST_PATH"
check "deleted playlist disappears" "$STATUS" "404"

echo "Original music playback"
ORIGINAL="$MEDIA/Album A/track 1.flac"
SIZE=$(wc -c < "$ORIGINAL" | tr -d ' ')
req POST "/items/$FOLDER/playback"
check "folders cannot issue a playback URL" "$STATUS" "404"
req POST "/items/$TRACK/playback"
PLAY_PATH=$(field .url)
check "playback reports direct mode with an expiry" "$STATUS $(field '.mode == "direct" and (.expiresAt | type == "string")')" "200 true"
case "$PLAY_PATH" in
  "/api/v1/items/$TRACK/stream?ticket="?*) ok "playback prepares a signed native URL" ;;
  *) bad "playback prepares a signed native URL" "missing or invalid playback URL"; exit 1 ;;
esac
PLAY_URL="$BASE$PLAY_PATH"
check "stream requires authorization" "$(read_media "$API/items/$TRACK/stream")" "401"
check "signed URL is scoped to one track" "$(read_media "$API/items/$OTHER_TRACK/stream?${PLAY_PATH#*\?}")" "401"
check "bearer can read the original track" "$(read_media -H "Authorization: Bearer $TOKEN" "$API/items/$TRACK/stream")" "200"
cmp -s "$ORIGINAL" "$PRIVATE_TMP/audio" && ok "playback preserves the original bytes" || bad "playback preserves the original bytes" "files differ"
check "two-byte probe returns the exact range" "$(read_media -H 'Range: bytes=0-1' "$PLAY_URL") $(media_header content-range) $(media_header content-length)" "206 bytes 0-1/$SIZE 2"
head -c 2 "$ORIGINAL" > "$PRIVATE_TMP/expected"
cmp -s "$PRIVATE_TMP/expected" "$PRIVATE_TMP/audio" && ok "probe contains the first two bytes" || bad "probe contains the first two bytes" "bytes differ"
check "signed HEAD reports complete metadata" \
  "$(read_media --head -H 'Range: bytes=0-1' "$PLAY_URL") $(media_header accept-ranges) $(media_header content-type) $(media_header content-length)" "200 bytes audio/flac $SIZE"
check "suffix seek reaches the original last bytes" \
  "$(read_media -H 'Range: bytes=-3' "$PLAY_URL") $(media_header content-range)" "206 bytes $((SIZE-3))-$((SIZE-1))/$SIZE"
tail -c 3 "$ORIGINAL" > "$PRIVATE_TMP/expected"
cmp -s "$PRIVATE_TMP/expected" "$PRIVATE_TMP/audio" && ok "seek returns the requested audio bytes" || bad "seek returns the requested audio bytes" "bytes differ"
check "seek past EOF reports the current size" \
  "$(read_media -H "Range: bytes=$SIZE-" "$PLAY_URL") $(media_header content-range) $(media_header content-length)" "416 bytes */$SIZE 0"
check "unmatched If-Range requests return the full file" \
  "$(read_media -H 'Range: bytes=0-1' -H 'If-Range: "older"' "$PLAY_URL") $(media_header content-length)" "200 $SIZE"

# ---------------------------------------------------------------- change detection
echo "Change detection"
req POST "/libraries/$LIB/scan"; wait_for_job "$(field .jobId)"
check "rescan finds nothing to do" \
  "$(printf '%s' "$JOB" | jq -r '.step.params | "\(.added) \(.updated) \(.removed) \(.unchanged)"')" "0 0 0 6"

rm "$MEDIA/Album A/track 3.flac"
req POST "/libraries/$LIB/scan"; wait_for_job "$(field .jobId)"
check "deleted file is removed from the index" "$(printf '%s' "$JOB" | jq -r '.step.params.removed')" "1"

req GET "/libraries/$LIB/music?artist=$ARTIST&album=$ALBUM"
check "removed track disappears from its album" "$(field '[.entries[].title] | join(",")')" "First Light,Interlude"

# ---------------------------------------------------------------- offline protection
echo "Unavailable library root"
mv "$MEDIA" "$GONE"
check "unavailable original files return 404" "$(read_media "$PLAY_URL")" "404"
req POST "/libraries/$LIB/scan"; wait_for_job "$(field .jobId)"
check "scan with every root missing fails" "$JOB_STATE" "failed"
check "failed job keeps no stale step text" "$(printf '%s' "$JOB" | jq -r '(.step|tostring) + " " + (.message|tostring) + " " + ((.error != null)|tostring)')" "null null true"
req GET "/libraries/$LIB"
check "no item was removed while the root was missing" "$(field .itemCount)" "5"
mv "$GONE" "$MEDIA"
req POST "/libraries/$LIB/scan"; wait_for_job "$(field .jobId)"
check "items are intact after the root returns" \
  "$JOB_STATE $(printf '%s' "$JOB" | jq -r '.step.params | "\(.added) \(.removed) \(.unchanged)"')" "succeeded 0 0 5"

# ---------------------------------------------------------------- admin pages
echo "Hardware, jobs, plugins"
req GET /system/hardware
check "hardware report has a performance tier" "$STATUS $(field '.perfTier != null')" "200 true"
echo "        ffmpeg: $(field '.ffmpeg | if .found then (.version + " (" + .source + ")") else "not found" end')"
echo "        verified encoders: $(field '[.encoders[] | select(.verified) | .id] | join(", ")')"
if [ -n "${PALVENI_SMOKE_FFMPEG_SOURCE:-}" ]; then
  check "ffmpeg and ffprobe found at the expected source" \
    "$(field '[.ffmpeg, .ffprobe] | map(select(.found) | .source) | join(" ")')" \
    "$PALVENI_SMOKE_FFMPEG_SOURCE $PALVENI_SMOKE_FFMPEG_SOURCE"
fi

req GET /jobs
check "job list is not empty" "$STATUS $(field '.jobs | length > 0')" "200 true"
req GET /plugins
check "plugin list is an array" "$STATUS $(field '.plugins | type')" "200 array"

# ---------------------------------------------------------------- skin packages
echo "Skin packages"
req GET /skins/registry
check "three official skin packages are available" "$STATUS $(field '[.skins[].id] | sort | join(",")')" "200 gear,ink,native"
req POST /skins/registry/native/install
check "official skin is hot-installed" "$STATUS $(field '.id + " " + .source')" "200 native official"
req GET /plugins
check "skin plugin is visible to the plugin manager" "$(field '[.plugins[] | select(.runtime == "none" and (.provides | index("skin")) != null)] | length')" "1"
STATUS=$(curl -sS -o "$PRIVATE_TMP/skin.json" -w '%{http_code}' "$API/skins/installed/native/skin.json")
check "skin manifest is public" "$STATUS $(jq -r .id "$PRIVATE_TMP/skin.json")" "200 native"
font=$(jq -r '.fonts[0].src' "$PRIVATE_TMP/skin.json")
check "font has a fixed public MIME type" "$(curl -sS -o /dev/null -w '%{http_code} %{content_type}' "$API/skins/installed/native/assets/$font")" "200 font/woff2"
check "catalog preview is public" "$(curl -sS -o /dev/null -w '%{http_code} %{content_type}' "$API/skins/registry/native/preview")" "200 image/png"
req DELETE /skins/installed/native
check "skin uninstall succeeds" "$STATUS" "204"
req GET /skins/installed/native/skin.json
check "uninstalled skin resources disappear immediately" "$STATUS" "404"

archive="$PRIVATE_TMP/native.tar"
check "official package is downloadable" "$(curl -sS -o "$archive" -w '%{http_code}' "$API/skins/registry/native/package")" "200"
post_archive() {
  local path="$1" file="$2"
  STATUS=$(curl -sS -X POST -H "authorization: Bearer $TOKEN" -H 'content-type: application/x-tar' \
    --data-binary "@$file" -o "$PRIVATE_TMP/body" -w '%{http_code}' "$API$path")
  BODY=$(cat "$PRIVATE_TMP/body")
}
post_archive /skins/registry/native/install "$archive"
check "downloaded official package matches pinned digest" "$STATUS $(field .id)" "200 native"
printf 'invalid package' > "$PRIVATE_TMP/invalid.tar"
post_archive /skins/registry/native/install "$PRIVATE_TMP/invalid.tar"
check "official install rejects a mismatched digest" "$STATUS" "400"
req DELETE /skins/installed/native
# Make an ordinary third-party package from the official static resources.
python3 - "$archive" "$PRIVATE_TMP/local.tar" <<'PYTHON'
import io, json, sys, tarfile
with tarfile.open(sys.argv[1]) as src, tarfile.open(sys.argv[2], 'w', format=tarfile.USTAR_FORMAT) as out:
    for member in src:
        if not member.isfile():
            continue
        data = src.extractfile(member).read()
        if member.name == 'plugin.toml':
            data = data.replace(b'com.palveni.skin.native', b'org.example.skin.sample')
        if member.name == 'skin.json':
            skin = json.loads(data)
            skin['id'] = 'sample'
            data = json.dumps(skin, ensure_ascii=False).encode()
        member.size = len(data)
        out.addfile(member, io.BytesIO(data))
PYTHON
post_archive /skins/upload "$PRIVATE_TMP/local.tar"
check "third-party upload installs without restarting" "$STATUS $(field '.id + " " + .source')" "201 sample upload"
post_archive /skins/upload "$PRIVATE_TMP/local.tar"
check "duplicate upload preserves existing installation" "$STATUS" "409"
req GET /skins/installed/sample/skin.json
check "uploaded manifest is available" "$STATUS $(field .id)" "200 sample"
req DELETE /skins/installed/sample
check "uploaded package can be removed" "$STATUS" "204"

# ---------------------------------------------------------------- events
echo "Events"
EVENTS="$(curl -sS -N --max-time 3 -H "authorization: Bearer $TOKEN" "$API/events" 2>/dev/null | head -c 400)"
case "$EVENTS" in *"event: ping"*) ok "event stream sends a ping on connect" ;; *) bad "event stream sends a ping on connect" "got: $(printf '%s' "$EVENTS" | head -c 80)" ;; esac

# ---------------------------------------------------------------- cleanup and logout
echo "Delete and logout"
req DELETE "/libraries/$LIB"
check "library is deleted" "$STATUS" "204"
[ -f "$MEDIA/Album A/track 1.flac" ] && ok "files on disk are untouched" || bad "files on disk are untouched" "a media file disappeared"
req GET "/libraries/$LIB"
check "deleted library is gone" "$STATUS" "404"
check "library removal makes its media URL unavailable" "$(read_media "$PLAY_URL")" "404"
req POST /auth/logout
check "logout succeeds" "$STATUS" "204"
req GET /auth/me
check "token no longer works" "$STATUS" "401"
check "logout revokes the signed media URL" "$(read_media "$PLAY_URL")" "401"

echo
echo "Login throttling"
TOKEN=""
failures=0
for _ in $(seq 1 8); do
  req POST /auth/login "{\"username\":\"$USERNAME\",\"password\":\"wrong-password\"}"
  [ "$STATUS" = 401 ] && failures=$((failures + 1))
done
check "first eight bad passwords are accounted separately" "$failures" "8"
req POST /auth/login "{\"username\":\"$USERNAME\",\"password\":\"wrong-password\"}"
check "login throttling returns a stable JSON error and retry duration" \
  "$STATUS $(field .error.code) $(awk 'tolower($1)=="retry-after:" && $2+0>=1 {ok=1} END {print ok+0}' "$PRIVATE_TMP/headers")" "429 too_many_requests 1"
echo "$PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
