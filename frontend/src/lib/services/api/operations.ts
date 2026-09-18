/** Faz 5 operasyon servisi: bloke, notlar, dosya ekleri, atama listeleri */

import { api, ApiError } from './client';
import type { AttachmentDto, BlockReasonDto } from '$lib/types';

export const blockReasonService = {
	list(): Promise<BlockReasonDto[]> {
		return api.get('/block-reasons');
	},
	create(input: { name: string; description?: string }): Promise<BlockReasonDto> {
		return api.post('/block-reasons', input);
	}
};

export const noteService = {
	add(executionId: string, note: string): Promise<{ ok: boolean }> {
		return api.post(`/process-executions/${executionId}/notes`, { note });
	}
};

export const blockService = {
	report(executionId: string, reasonId: string, description?: string) {
		return api.post(`/process-executions/${executionId}/block`, {
			reason_id: reasonId,
			description
		});
	},
	resolve(executionId: string, resolutionNote?: string) {
		return api.post(
			`/process-executions/${executionId}/unblock`,
			resolutionNote ? { resolution_note: resolutionNote } : undefined
		);
	}
};

/** Multipart yükleme — client.ts'in JSON wrapper'ı kullanılmaz. */
async function uploadFile(
	path: string,
	file: File,
	fields: Record<string, string>
): Promise<AttachmentDto> {
	const form = new FormData();
	for (const [k, v] of Object.entries(fields)) form.append(k, v);
	form.append('file', file);

	const response = await fetch(`/api/v1${path}`, {
		method: 'POST',
		credentials: 'same-origin',
		body: form
	});
	const body: unknown = await response.json().catch(() => null);
	if (!response.ok) {
		const errorBody = body as { code?: string; message?: string } | null;
		throw new ApiError(
			errorBody?.code ?? 'INTERNAL_ERROR',
			errorBody?.message ?? 'Yükleme başarısız.',
			response.status
		);
	}
	return body as AttachmentDto;
}

export const attachmentService = {
	upload(entityType: string, entityId: string, file: File): Promise<AttachmentDto> {
		return uploadFile('/attachments', file, { entity_type: entityType, entity_id: entityId });
	},
	list(entityType: string, entityId: string): Promise<AttachmentDto[]> {
		return api.get(`/attachments?entity_type=${entityType}&entity_id=${entityId}`);
	},
	downloadUrl(attachmentId: string): string {
		return `/api/v1/attachments/${attachmentId}/download`;
	},
	remove(attachmentId: string): Promise<{ ok: boolean }> {
		return api.del(`/attachments/${attachmentId}`);
	}
};

/** Atama UI'ı için kullanıcı/takım listeleri (ADMIN+PM erişimi). */
export const directoryService = {
	users(workspaceId: string): Promise<{ id: string; full_name: string; role: string }[]> {
		return api.get(`/workspaces/${workspaceId}/users`);
	},
	teams(): Promise<{ id: string; name: string }[]> {
		return api.get('/teams');
	}
};
