import { redirect, type Handle } from '@sveltejs/kit';

const SESSION_COOKIE = 'atolye_session';

/**
 * Auth-aware routing (docs/architecture.md §3.1):
 * - Oturum cookie'si olmayan istekler korumalı sayfalardan /login'e yönlendirilir.
 * - Cookie geçerliliği backend'de (/auth/me) her istekte doğrulanır;
 *   buradaki kontrol yalnızca hızlı ön süzgeçtir.
 */
export const handle: Handle = async ({ event, resolve }) => {
	const path = event.url.pathname;
	const isLoginPage = path === '/login';
	const hasSessionCookie = event.cookies.get(SESSION_COOKIE) !== undefined;

	// Statik asset / internal istekler dokunulmaz
	if (path.startsWith('/_app/') || path.startsWith('/favicon')) {
		return resolve(event);
	}

	if (!hasSessionCookie && !isLoginPage) {
		throw redirect(302, '/login');
	}

	return resolve(event);
};
