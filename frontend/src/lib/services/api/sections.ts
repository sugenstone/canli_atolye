/** Section servisi — endpoint listesi: docs/architecture.md §15 */

import { api } from './client';
import type { SectionDto, SectionTreeResponse } from '$lib/types';

export interface CreateSectionInput {
	name: string;
	code?: string;
	type?: string;
	parent_id?: string | null;
}

export interface BulkSectionInput {
	parent_id?: string | null;
	start_index: number;
	count: number;
	name_format: string;
	code_format?: string;
	type?: string;
}

export interface CloneSectionInput {
	count: number;
	name_format?: string;
	code_format?: string;
}

export const sectionService = {
	tree(projectId: string): Promise<SectionTreeResponse> {
		return api.get(`/projects/${projectId}/sections/tree`);
	},

	create(projectId: string, input: CreateSectionInput): Promise<SectionDto> {
		return api.post(`/projects/${projectId}/sections`, input);
	},

	update(
		sectionId: string,
		patch: { name?: string; code?: string | null; type?: string | null }
	): Promise<SectionDto> {
		return api.patch(`/sections/${sectionId}`, patch);
	},

	/** Soft delete; alt bölümü olanlar 409 alır. */
	remove(sectionId: string): Promise<{ ok: boolean }> {
		return api.del(`/sections/${sectionId}`);
	},

	bulkCreate(projectId: string, input: BulkSectionInput): Promise<SectionDto[]> {
		return api.post(`/projects/${projectId}/sections/bulk`, input);
	},

	/** Alt ağacı `count` kez kopyalar; ad otomatik türetilir ("1. Kat" → "2. Kat"). */
	clone(sectionId: string, input: CloneSectionInput): Promise<SectionDto[]> {
		return api.post(`/sections/${sectionId}/clone`, input);
	}
};
