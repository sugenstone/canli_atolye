/** Auth servisi — endpoint listesi: docs/architecture.md §15 */

import { api } from './client';
import type { MeResponse } from '$lib/types';

export const authService = {
	login(email: string, password: string): Promise<{ ok: boolean }> {
		return api.post('/auth/login', { email, password });
	},

	logout(): Promise<{ ok: boolean }> {
		return api.post('/auth/logout');
	},

	me(): Promise<MeResponse> {
		return api.get('/auth/me');
	}
};
