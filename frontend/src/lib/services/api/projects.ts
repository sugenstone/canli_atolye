/** Proje servisi */

import { api } from './client';
import type { ProjectDto, ProjectStatus } from '$lib/types';

export interface CreateProjectInput {
	name: string;
	code: string;
	description?: string;
}

export const projectService = {
	list(status?: ProjectStatus): Promise<ProjectDto[]> {
		const query = status ? `?status=${status}` : '';
		return api.get(`/projects${query}`);
	},

	create(input: CreateProjectInput): Promise<ProjectDto> {
		return api.post('/projects', input);
	},

	get(id: string): Promise<ProjectDto> {
		return api.get(`/projects/${id}`);
	},

	update(id: string, patch: Partial<CreateProjectInput> & { status?: ProjectStatus }): Promise<ProjectDto> {
		return api.patch(`/projects/${id}`, patch);
	},

	archive(id: string): Promise<ProjectDto> {
		return api.post(`/projects/${id}/archive`);
	}
};
