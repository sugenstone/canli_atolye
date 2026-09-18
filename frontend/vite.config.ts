import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		proxy: {
			// Dev'de API çağrıları aynı origin üzerinden backend'e gider → cookie/CORS sorunu yok.
			'/api': 'http://127.0.0.1:8080'
		}
	}
});
