/**
 * Merkezi API istemcisi (docs/architecture.md §3.4).
 * Component'lerde doğrudan `fetch` kullanılmaz; her şey buradan geçer.
 * Hatalar ApiError olarak normalize edilir.
 */

export class ApiError extends Error {
	constructor(
		public code: string,
		message: string,
		public status: number
	) {
		super(message);
		this.name = 'ApiError';
	}
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
	const hasBody = options.body !== undefined && options.body !== null;
	const response = await fetch(`/api/v1${path}`, {
		credentials: 'same-origin',
		...options,
		headers: {
			// Content-Type yalnızca gövdeli isteklerde: boş gövde + JSON header
			// Axum tarafında parse hatası (400) üretir.
			...(hasBody ? { 'Content-Type': 'application/json' } : {}),
			...options.headers
		}
	});

	// Boş gövde (204 / bazı action endpoint'leri)
	if (response.status === 204) {
		return undefined as T;
	}

	const body: unknown = await response.json().catch(() => null);

	if (!response.ok) {
		const errorBody = body as { code?: string; message?: string } | null;
		throw new ApiError(
			errorBody?.code ?? 'INTERNAL_ERROR',
			errorBody?.message ?? 'Beklenmeyen bir hata oluştu.',
			response.status
		);
	}

	return body as T;
}

export const api = {
	get: <T>(path: string) => request<T>(path),
	post: <T>(path: string, data?: unknown) =>
		request<T>(path, {
			method: 'POST',
			body: data === undefined ? undefined : JSON.stringify(data)
		}),
	put: <T>(path: string, data: unknown) =>
		request<T>(path, { method: 'PUT', body: JSON.stringify(data) }),
	patch: <T>(path: string, data: unknown) =>
		request<T>(path, { method: 'PATCH', body: JSON.stringify(data) }),
	del: <T>(path: string) => request<T>(path, { method: 'DELETE' })
};
