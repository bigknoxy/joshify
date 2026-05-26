# Spotify API Guidelines for joshify

This document contains the Spotify Web API guidelines that must be followed when working on joshify.

## Spotify API Rules

### OpenAPI Specification
- **Reference**: Always refer to the Spotify OpenAPI specification at https://developer.spotify.com/reference/web-api/open-api-schema.yaml for all endpoint paths, parameters, and response schemas.
- **Do not guess** endpoints or field names - always check the spec.

### Authorization Flow
- **Use**: Authorization Code with PKCE flow (https://developer.spotify.com/documentation/web-api/tutorials/code-pkce-flow) for any user-specific data.
- **Alternative**: If the app has a secure backend, the Authorization Code flow (https://developer.spotify.com/documentation/web-api/tutorials/code-flow) is also acceptable.
- **For public data only**: Client Credentials can be used for public, non-user data.
- **Never use**: Implicit Grant flow (it is deprecated).

### Redirect URIs
- **Always use HTTPS** redirect URIs (except http://127.0.0.1 for local development).
- **Never use**: http://localhost or wildcard URIs.
- **Reference**: See https://developer.spotify.com/documentation/web-api/concepts/redirect_uri for requirements.

### Scopes
- **Request minimum**: Only request the minimum scopes (https://developer.spotify.com/documentation/web-api/concepts/scopes) needed for the features being built.
- **Do not**: Request broad scopes preemptively.

### Token Management
- **Store securely**: Store tokens securely.
- **Never expose**: Never expose the Client Secret in client-side code.
- **Implement refresh**: Implement token refresh (https://developer.spotify.com/documentation/web-api/tutorials/refreshing-tokens) logic so the app does not break when access tokens expire.

### Rate Limits
- **Exponential backoff**: Implement exponential backoff and respect the Retry-After header when receiving HTTP 429 responses.
- **Do not**: Retry immediately or in tight loops.

### Deprecated Endpoints
- **Do not use**: Do not use deprecated endpoints.
- **Prefer**:
  - `/playlists/{id}/items` over `/playlists/{id}/tracks`
  - `/me/library` over the type-specific library endpoints

### Error Handling
- **Handle all codes**: Handle all HTTP error codes documented in the OpenAPI schema.
- **Read errors**: Read the returned error message and use it to provide meaningful feedback to the user.

### Developer Terms of Service
- **Comply**: Comply with the Spotify Developer Terms (https://developer.spotify.com/terms).
- **In particular**:
  - Do not cache Spotify content beyond what is needed for immediate use
  - Always attribute content to Spotify
  - Do not use the API to train machine learning models on Spotify data

## Application to joshify

Current implementation uses:
- Authorization Code flow with PKCE (via rspotify)
- Token refresh implemented
- Rate limiting handled

Areas to verify/improve:
1. Check if any deprecated endpoints are being used
2. Verify error handling follows Spotify's documented error codes
3. Ensure scopes are minimal for each feature
4. Review caching strategy against ToS requirements
