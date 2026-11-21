import Foundation

public enum PalveniClientError: Error, Equatable, Sendable {
    case invalidServerURL
    case invalidAccessToken
    case invalidResponse
    case httpStatus(Int)
}

public enum PlaybackUpdateResult: Equatable, Sendable {
    case saved(MediaState)
    /// Another session updated this item. Contains the state read after the conflict.
    case conflict(current: MediaState)
}

public struct PalveniClient: Sendable {
    private let serverURL: URL
    private let accessToken: String
    private static let session: URLSession = {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.urlCache = nil
        configuration.urlCredentialStorage = nil
        configuration.httpCookieStorage = nil
        return URLSession(configuration: configuration)
    }()

    /// Supply the installation root, including its reverse-proxy prefix when present.
    public init(serverURL: URL, accessToken: String) throws {
        guard let parts = URLComponents(url: serverURL, resolvingAgainstBaseURL: true),
              ["http", "https"].contains(parts.scheme?.lowercased() ?? ""),
              let host = parts.host, !host.isEmpty,
              parts.user == nil, parts.password == nil, parts.query == nil, parts.fragment == nil
        else { throw PalveniClientError.invalidServerURL }
        guard !accessToken.isEmpty, accessToken.utf8.allSatisfy({ (33...126).contains($0) })
        else { throw PalveniClientError.invalidAccessToken }

        self.serverURL = serverURL.absoluteURL
        self.accessToken = accessToken
    }

    private var itemsURL: URL { serverURL.appending(path: "api/v1/me/items") }

    private func itemURL(_ itemID: UUID) -> URL {
        itemsURL.appending(component: itemID.uuidString.lowercased())
    }

    private func authenticatedRequest(_ url: URL) -> URLRequest {
        var request = URLRequest(url: url, cachePolicy: .reloadIgnoringLocalCacheData)
        request.setValue("Bearer \(accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        return request
    }

    public func mediaState(for itemID: UUID) async throws -> MediaState {
        try await send(authenticatedRequest(itemURL(itemID)))
    }

    public func playbackState(for itemID: UUID) async throws -> MediaState {
        try await mediaState(for: itemID)
    }

    /// Sets the desired state while preserving the item's playback version and position.
    public func setFavorite(_ favorite: Bool, for itemID: UUID) async throws -> MediaState {
        let request = try putRequest(FavoriteUpdate(favorite: favorite),
                                     to: itemURL(itemID).appending(component: "favorite"))
        return try await send(request)
    }

    /// Supply the previous page's nextBefore to continue in most-recently-added order.
    public func favorites(before: Int64? = nil, limit: Int? = nil) async throws -> PersonalItemPage {
        var query = [URLQueryItem(name: "view", value: "favorites")]
        if let before { query.append(URLQueryItem(name: "before", value: String(before))) }
        if let limit { query.append(URLQueryItem(name: "limit", value: String(limit))) }
        return try await send(authenticatedRequest(itemsURL.appending(queryItems: query)))
    }

    /// Pass the playbackVersion observed before the local edit, including when retrying it.
    public func reportPlayback(for itemID: UUID, positionMs: Int64, completed: Bool,
                               expectedVersion: Int64) async throws -> PlaybackUpdateResult {
        let update = PlaybackUpdate(
            expectedVersion: expectedVersion, positionMs: positionMs, completed: completed
        )
        let request = try putRequest(update, to: itemURL(itemID).appending(component: "progress"))
        do {
            return .saved(try await send(request))
        } catch PalveniClientError.httpStatus(409) {
            return .conflict(current: try await playbackState(for: itemID))
        }
    }

    private func putRequest(_ body: some Encodable, to url: URL) throws -> URLRequest {
        var request = authenticatedRequest(url)
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(body)
        return request
    }

    private func send<Response: Decodable>(_ request: URLRequest) async throws -> Response {
        let (data, response) = try await Self.session.data(for: request, delegate: RedirectPolicy.shared)
        guard let response = response as? HTTPURLResponse else { throw PalveniClientError.invalidResponse }
        guard response.statusCode == 200 else { throw PalveniClientError.httpStatus(response.statusCode) }
        return try JSONDecoder().decode(Response.self, from: data)
    }
}

private final class RedirectPolicy: NSObject, URLSessionTaskDelegate {
    static let shared = RedirectPolicy()

    // The authenticated request belongs to the configured installation.
    func urlSession(_ session: URLSession, task: URLSessionTask,
                    willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest) async -> URLRequest? {
        nil
    }
}
