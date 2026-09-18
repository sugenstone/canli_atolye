/**
 * Merkezi realtime servisi (docs/architecture.md §3.5, §16).
 * Tek EventSource bağlantısı; her sayfa kendi aboneliğini açar-kapar.
 * EventSource otomatik reconnect yapar; event geldiğinde invalidation tetiklenir.
 */

const PROCESS_EVENTS = [
	'process.created',
	'process.ready',
	'process.assigned',
	'process.started',
	'process.paused',
	'process.resumed',
	'process.blocked',
	'process.unblocked',
	'process.completed',
	'process.cancelled',
	'process.note.added'
] as const;

export type RealtimeHandler = (eventName: string, payload: unknown) => void;

export interface RealtimeSubscription {
	close: () => void;
}

/** Projeye abone ol; dönen nesneyle bağlantı kapatılır. */
export function subscribeProject(
	projectId: string,
	onEvent: RealtimeHandler
): RealtimeSubscription {
	const source = new EventSource(`/api/v1/projects/${projectId}/events/stream`);

	for (const name of PROCESS_EVENTS) {
		source.addEventListener(name, (event) => {
			let payload: unknown = null;
			try {
				payload = JSON.parse((event as MessageEvent).data);
			} catch {
				payload = null;
			}
			onEvent(name, payload);
		});
	}

	source.onerror = () => {
		// EventSource kendi yeniden bağlanır; konsol kaydı yeterli
		console.warn('[realtime] bağlantı koptu, yeniden bağlanılıyor…');
	};

	return {
		close: () => source.close()
	};
}
