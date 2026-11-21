// Generated from the Palveni OpenAPI document by Tools/generate_models.py.
import Foundation

public enum ItemKind: String, Decodable, Sendable {
    case folder
    case video
    case track
    case chapter
    case episode
    case photo
    case book
    case comic
}

public struct MediaState: Decodable, Equatable, Sendable {
    public let favorite: Bool
    public let stars: Int32
    public let positionMs: Int64
    public let completed: Bool
    public let playCount: Int64
    public let lastPlayedAt: String?
    public let playbackVersion: Int64
}

public struct PersonalItem: Decodable, Equatable, Sendable {
    public let itemId: UUID
    public let libraryId: UUID
    public let title: String
    public let kind: ItemKind
    public let state: MediaState
}

public struct PersonalItemPage: Decodable, Equatable, Sendable {
    public let items: [PersonalItem]
    public let nextBefore: Int64?
}

struct PlaybackUpdate: Encodable, Equatable, Sendable {
    let expectedVersion: Int64
    let positionMs: Int64
    let completed: Bool
}

struct FavoriteUpdate: Encodable, Equatable, Sendable {
    let favorite: Bool
}
