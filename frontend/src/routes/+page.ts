import { redirect } from '@sveltejs/kit';

/** Kök → dashboard yönlendirmesi */
export function load() {
	throw redirect(302, '/dashboard');
}
