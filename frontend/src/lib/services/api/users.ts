/** Kullanıcı ve takım yönetimi servisi (ADMIN) */

import { api } from './client';
import type { Role } from '$lib/types';

export interface UserDto {
	id: string;
	email: string;
	full_name: string;
	role: Role;
	active: boolean;
	created_at: string;
}

export const userService = {
	list(workspaceId: string): Promise<UserDto[]> {
		return api.get(`/workspaces/${workspaceId}/users`);
	},

	create(
		workspaceId: string,
		input: { email: string; password: string; full_name: string; role: Role }
	): Promise<UserDto> {
		return api.post(`/workspaces/${workspaceId}/users`, input);
	},

	update(
		userId: string,
		patch: { full_name?: string; role?: Role; active?: boolean }
	): Promise<UserDto> {
		return api.patch(`/users/${userId}`, patch);
	},

	/** Pasifleştirme (soft) */
	deactivate(userId: string): Promise<UserDto> {
		return api.del(`/users/${userId}`);
	}
};

export interface TeamDto {
	id: string;
	name: string;
	description?: string | null;
}

export interface TeamMemberDto {
	team_id: string;
	user_id: string;
	role: string;
}

export const teamAdminService = {
	list(): Promise<TeamDto[]> {
		return api.get('/teams');
	},

	create(input: { name: string; description?: string }): Promise<TeamDto> {
		return api.post('/teams', input);
	},

	listMembers(teamId: string): Promise<TeamMemberDto[]> {
		return api.get(`/teams/${teamId}/members`);
	},

	addMember(teamId: string, userId: string): Promise<{ ok: boolean }> {
		return api.post(`/teams/${teamId}/members/${userId}`, { user_id: userId });
	},

	removeMember(teamId: string, userId: string): Promise<{ ok: boolean }> {
		return api.del(`/teams/${teamId}/members/${userId}`);
	}
};

export const ROLE_LABELS: Record<Role, string> = {
	ADMIN: 'Yönetici',
	PROJECT_MANAGER: 'Proje Yöneticisi',
	TEAM_LEADER: 'Takım Lideri',
	WORKER: 'Çalışan',
	VIEWER: 'İzleyici'
};
