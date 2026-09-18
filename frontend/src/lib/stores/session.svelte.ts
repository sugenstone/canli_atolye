/**
 * Oturum state'i — feature bazlı küçük store örneği (docs/architecture.md §3.3).
 * Server state (me) burada tutulur; UI state'i component'lerde kalır.
 */

import { authService } from '$lib/services/api/auth';
import type { MeResponse } from '$lib/types';

class SessionStore {
	me = $state<MeResponse | null>(null);
	loading = $state(true);

	/** Oturumu backend'den yükle; 401 → null. */
	async load(): Promise<MeResponse | null> {
		this.loading = true;
		try {
			this.me = await authService.me();
		} catch {
			this.me = null;
		} finally {
			this.loading = false;
		}
		return this.me;
	}

	clear() {
		this.me = null;
	}

	async logout() {
		try {
			await authService.logout();
		} finally {
			this.clear();
		}
	}

	get isAuthenticated(): boolean {
		return this.me !== null;
	}

	get isAdmin(): boolean {
		return this.me?.user.role === 'ADMIN';
	}
}

export const session = new SessionStore();
