# [SDK] API Client Core - HTTP Client with Authentication and Error Handling

## 🎯 Problem Statement

Currently, there's no shared SDK for API communication between web and mobile clients. This leads to:
- Duplicated API logic across platforms
- Inconsistent error handling
- No centralized authentication management
- Difficult to maintain API changes
- No type safety for API responses

## 💡 Solution

Create a core TypeScript API client that handles HTTP requests, authentication, error handling, and provides type-safe methods for all API endpoints. This client will work in both browser and React Native environments.

## 📁 Files to Create/Modify

### New Files
```
sdk/src/client/ApiClient.ts
sdk/src/client/types.ts
sdk/src/client/errors.ts
sdk/src/client/interceptors.ts
```

### Modified Files
```
sdk/src/index.ts
sdk/package.json
```

## 🔧 Technical Implementation

### ApiClient Class
```typescript
export class ApiClient {
  private baseURL: string;
  private timeout: number;
  private authToken?: string;

  constructor(config: ApiClientConfig) {
    this.baseURL = config.baseURL;
    this.timeout = config.timeout || 30000;
  }

  async get<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('GET', path, undefined, options);
  }

  async post<T>(path: string, data?: unknown, options?: RequestOptions): Promise<T> {
    return this.request<T>('POST', path, data, options);
  }

  private async request<T>(
    method: string,
    path: string,
    data?: unknown,
    options?: RequestOptions
  ): Promise<T> {
    const url = `${this.baseURL}${path}`;
    const headers = this.buildHeaders(options);

    try {
      const response = await fetch(url, {
        method,
        headers,
        body: data ? JSON.stringify(data) : undefined,
        signal: options?.signal,
      });

      if (!response.ok) {
        throw await this.handleErrorResponse(response);
      }

      return await response.json();
    } catch (error) {
      throw this.handleNetworkError(error);
    }
  }

  setAuthToken(token: string): void {
    this.authToken = token;
  }

  clearAuthToken(): void {
    this.authToken = undefined;
  }
}
```

### Error Handling
```typescript
export class ApiError extends Error {
  constructor(
    public statusCode: number,
    public code: string,
    message: string,
    public details?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export class NetworkError extends Error {
  constructor(message: string, public originalError?: Error) {
    super(message);
    this.name = 'NetworkError';
  }
}
```

## ✅ Acceptance Criteria

### Core Functionality
- [ ] ApiClient class with GET, POST, PUT, DELETE methods
- [ ] Supports both browser and React Native environments
- [ ] Configurable base URL and timeout
- [ ] Request/response interceptors
- [ ] Automatic JSON parsing
- [ ] Request cancellation via AbortSignal

### Authentication
- [ ] Token management (set/get/clear)
- [ ] Automatic token injection in headers
- [ ] Token refresh on 401 responses
- [ ] Secure token storage integration

### Error Handling
- [ ] Network errors properly caught and wrapped
- [ ] HTTP errors parsed and typed
- [ ] Retry logic for transient failures
- [ ] Clear error messages for debugging

### Type Safety
- [ ] Generic type parameters for responses
- [ ] TypeScript interfaces for all methods
- [ ] Proper error types
- [ ] Request/response type definitions

### Testing
- [ ] Unit tests for all methods (>80% coverage)
- [ ] Mock fetch for testing
- [ ] Test error scenarios
- [ ] Test authentication flow
- [ ] Test request cancellation

## 🧪 Testing Strategy

### Unit Tests
```typescript
describe('ApiClient', () => {
  it('makes GET requests', async () => {
    const client = new ApiClient({ baseURL: 'https://api.example.com' });
    const data = await client.get('/users');
    expect(data).toBeDefined();
  });

  it('handles 404 errors', async () => {
    const client = new ApiClient({ baseURL: 'https://api.example.com' });
    await expect(client.get('/not-found')).rejects.toThrow(ApiError);
  });

  it('injects auth token', async () => {
    const client = new ApiClient({ baseURL: 'https://api.example.com' });
    client.setAuthToken('test-token');
    // Verify Authorization header is set
  });
});
```

### Integration Tests
- Test against real API endpoints (testnet)
- Test authentication flow end-to-end
- Test error handling with real errors
- Test network failures

## 🔗 Dependencies

**Depends on**: SDK Package Initialization
**Required by**: All SDK API modules (Corridors, Anchors, Assets, Analytics)

## ⏱️ Estimated Effort

**Total: 6-8 hours**
- Core implementation: 2 hours
- Error handling: 1 hour
- Authentication: 1 hour
- Unit tests: 2 hours
- Integration tests: 1 hour
- Documentation: 1 hour

## ✔️ Definition of Done

- [ ] ApiClient implemented with all HTTP methods
- [ ] Works in browser and React Native
- [ ] Authentication token management working
- [ ] Error handling comprehensive
- [ ] Unit tests pass (>80% coverage)
- [ ] Integration tests pass
- [ ] TypeScript types complete
- [ ] Documentation complete
- [ ] Code reviewed and approved
