import Foundation
import PalveniClient
import XCTest

final class PlaybackTests: XCTestCase {
    func testSessionRequiresAnHTTPInstallationAndHeaderSafeBearer() throws {
        let root = try XCTUnwrap(URL(string: "https://example.invalid/palveni/"))
        _ = try PalveniClient(serverURL: root, accessToken: "pvt_fixture")
        for token in ["", "a b", "line\r\nbreak"] {
            XCTAssertThrowsError(try PalveniClient(serverURL: root, accessToken: token)) {
                XCTAssertEqual($0 as? PalveniClientError, .invalidAccessToken)
            }
        }
        for text in ["file:///a", "https://x:pw@example.invalid/", "https://example.invalid/?token=x"] {
            let url = try XCTUnwrap(URL(string: text))
            XCTAssertThrowsError(try PalveniClient(serverURL: url, accessToken: "pvt_fixture")) {
                XCTAssertEqual($0 as? PalveniClientError, .invalidServerURL)
            }
        }
    }

    private func sessions() throws -> (PalveniClient, PalveniClient) {
        let env = ProcessInfo.processInfo.environment
        guard let rawURL = env["PALVENI_SWIFT_TEST_URL"] else {
            throw XCTSkip("Run Tools/smoke-playback.py with a local palveni binary.")
        }
        let url = try XCTUnwrap(URL(string: rawURL))
        let first = try PalveniClient(serverURL: url, accessToken: XCTUnwrap(env["PALVENI_SWIFT_TEST_TOKEN_A"]))
        let second = try PalveniClient(serverURL: url, accessToken: XCTUnwrap(env["PALVENI_SWIFT_TEST_TOKEN_B"]))
        return (first, second)
    }

    private func fixtureID(_ key: String) throws -> UUID {
        try XCTUnwrap(ProcessInfo.processInfo.environment[key].flatMap(UUID.init(uuidString:)))
    }

    func testFavoritesAcrossSessions() async throws {
        let (first, second) = try sessions()
        let a = try fixtureID("PALVENI_SWIFT_FAVORITE_A")
        let b = try fixtureID("PALVENI_SWIFT_FAVORITE_B")
        let initial = try await second.mediaState(for: a)
        XCTAssertFalse(initial.favorite)
        let added = try await first.setFavorite(true, for: a)
        XCTAssertTrue(added.favorite)
        XCTAssertEqual(added.playbackVersion, initial.playbackVersion)

        let resume = try await second.reportPlayback(for: a, positionMs: 3_000, completed: false,
                                                     expectedVersion: initial.playbackVersion)
        let afterPlayback = try await first.mediaState(for: a)
        XCTAssertEqual(resume, .saved(afterPlayback))
        XCTAssertTrue(afterPlayback.favorite)
        XCTAssertEqual(afterPlayback.positionMs, 3_000)
        _ = try await second.setFavorite(true, for: b)
        let retry = try await first.setFavorite(true, for: a)
        XCTAssertEqual(retry, afterPlayback)

        let newest = try await second.favorites(limit: 1)
        XCTAssertEqual(newest.items.map(\.itemId), [b])
        let cursor = try XCTUnwrap(newest.nextBefore)
        let older = try await first.favorites(before: cursor, limit: 1)
        XCTAssertEqual(older.items.map(\.itemId), [a])
        XCTAssertEqual(older.items.first?.title, "海边来信")
        XCTAssertEqual(older.items.first?.kind, .track)
        XCTAssertEqual(older.items.first?.state, afterPlayback)
        XCTAssertNil(older.nextBefore)

        let removed = try await second.setFavorite(false, for: a)
        let repeated = try await first.setFavorite(false, for: a)
        let observed = try await first.mediaState(for: a)
        XCTAssertEqual(removed, repeated)
        XCTAssertEqual(removed, observed)
        XCTAssertFalse(observed.favorite)
        XCTAssertEqual(observed.positionMs, afterPlayback.positionMs)
        XCTAssertEqual(observed.playbackVersion, afterPlayback.playbackVersion)
        let remaining = try await first.favorites()
        XCTAssertEqual(remaining.items.map(\.itemId), [b])
        do {
            _ = try await first.setFavorite(true, for: UUID())
            XCTFail("A missing item was favorited")
        } catch let error as PalveniClientError {
            XCTAssertEqual(error, .httpStatus(404))
        }
    }

    func testPlaybackAcrossSessions() async throws {
        let (first, second) = try sessions()
        let item = try fixtureID("PALVENI_SWIFT_TEST_ITEM")
        let initial = try await first.playbackState(for: item)
        let old = try await second.playbackState(for: item)
        XCTAssertEqual(initial, old)
        XCTAssertEqual(initial.playbackVersion, 0)

        let pause = try await first.reportPlayback(for: item, positionMs: 7_500, completed: false,
                                                   expectedVersion: initial.playbackVersion)
        let resume = try await second.playbackState(for: item)
        XCTAssertEqual(pause, .saved(resume))
        XCTAssertEqual(resume.positionMs, 7_500)
        XCTAssertGreaterThan(resume.playbackVersion, initial.playbackVersion)

        let stale = try await second.reportPlayback(for: item, positionMs: 1_000, completed: false,
                                                    expectedVersion: old.playbackVersion)
        XCTAssertEqual(stale, .conflict(current: resume))
        let afterConflict = try await first.playbackState(for: item)
        XCTAssertEqual(afterConflict, resume)

        let finish = try await second.reportPlayback(for: item, positionMs: 10_000, completed: true,
                                                     expectedVersion: resume.playbackVersion)
        let retry = try await second.reportPlayback(for: item, positionMs: 10_000, completed: true,
                                                    expectedVersion: resume.playbackVersion)
        let finished = try await first.playbackState(for: item)
        XCTAssertEqual(finish, .saved(finished))
        XCTAssertEqual(retry, finish)
        XCTAssertTrue(finished.completed)
        XCTAssertEqual(finished.positionMs, 0)
        XCTAssertEqual(finished.playCount, 1)
        XCTAssertNotNil(finished.lastPlayedAt)

        let origin = try XCTUnwrap(ProcessInfo.processInfo.environment["PALVENI_SWIFT_TEST_URL"])
        let revoked = try PalveniClient(serverURL: XCTUnwrap(URL(string: origin)), accessToken: "invalid-fixture")
        do {
            _ = try await revoked.playbackState(for: item)
            XCTFail("Unauthenticated playback state request succeeded")
        } catch let error as PalveniClientError {
            XCTAssertEqual(error, .httpStatus(401))
        }
    }
}
