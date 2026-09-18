/**
 * Status görsel sistemi — tek kaynak (docs/architecture.md §4.3).
 *
 * Kurallar:
 * - Renk sınıfları tam string olarak literal tutulur (Tailwind taraması için).
 * - Sadece renge güvenilmez: ikon + metin her zaman birlikte kullanılır.
 * - Component'lerde bu map dışında status rengi yazılmaz.
 */

export interface StatusToken {
	label: string;
	/** Badge gövdesi */
	badgeClass: string;
	/** Küçük durum noktası */
	dotClass: string;
}

/** Proje durumları (Faz 1) */
export const PROJECT_STATUS: Record<string, StatusToken> = {
	DRAFT: {
		label: 'Taslak',
		badgeClass: 'bg-gray-100 text-gray-700 border border-gray-300 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-600',
		dotClass: 'bg-gray-500'
	},
	ACTIVE: {
		label: 'Aktif',
		badgeClass: 'bg-blue-100 text-blue-700 border border-blue-300 dark:bg-blue-900/40 dark:text-blue-300 dark:border-blue-700',
		dotClass: 'bg-in-progress'
	},
	PAUSED: {
		label: 'Duraklatıldı',
		badgeClass: 'bg-orange-100 text-orange-700 border border-orange-300 dark:bg-orange-900/40 dark:text-orange-300 dark:border-orange-700',
		dotClass: 'bg-paused'
	},
	COMPLETED: {
		label: 'Tamamlandı',
		badgeClass: 'bg-green-100 text-green-700 border border-green-300 dark:bg-green-900/40 dark:text-green-300 dark:border-green-700',
		dotClass: 'bg-completed'
	},
	CANCELLED: {
		label: 'İptal',
		badgeClass: 'bg-slate-100 text-slate-700 border border-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:border-slate-600',
		dotClass: 'bg-cancelled'
	},
	ARCHIVED: {
		label: 'Arşiv',
		badgeClass: 'bg-slate-100 text-slate-600 border border-slate-300 dark:bg-slate-800/60 dark:text-slate-400 dark:border-slate-700',
		dotClass: 'bg-project-archived'
	}
};

/** Süreç durumları (Faz 4 — Process Engine ile devreye girer) */
export const PROCESS_STATUS: Record<string, StatusToken> = {
	PENDING: {
		label: 'Bekliyor',
		badgeClass: 'bg-gray-100 text-gray-700 border border-gray-300 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-600',
		dotClass: 'bg-status-pending'
	},
	READY: {
		label: 'Hazır',
		badgeClass: 'bg-amber-100 text-amber-800 border border-amber-300 dark:bg-amber-900/40 dark:text-amber-300 dark:border-amber-700',
		dotClass: 'bg-status-ready'
	},
	IN_PROGRESS: {
		label: 'Devam Ediyor',
		badgeClass: 'bg-blue-100 text-blue-700 border border-blue-300 dark:bg-blue-900/40 dark:text-blue-300 dark:border-blue-700',
		dotClass: 'bg-in-progress'
	},
	PAUSED: {
		label: 'Duraklatıldı',
		badgeClass: 'bg-orange-100 text-orange-700 border border-orange-300 dark:bg-orange-900/40 dark:text-orange-300 dark:border-orange-700',
		dotClass: 'bg-paused'
	},
	BLOCKED: {
		label: 'Bloke',
		badgeClass: 'bg-red-100 text-red-700 border border-red-300 dark:bg-red-900/40 dark:text-red-300 dark:border-red-700',
		dotClass: 'bg-blocked'
	},
	COMPLETED: {
		label: 'Tamamlandı',
		badgeClass: 'bg-green-100 text-green-700 border border-green-300 dark:bg-green-900/40 dark:text-green-300 dark:border-green-700',
		dotClass: 'bg-completed'
	},
	CANCELLED: {
		label: 'İptal',
		badgeClass: 'bg-slate-100 text-slate-700 border border-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:border-slate-600',
		dotClass: 'bg-cancelled'
	}
};

/** Bilinmeyen durum için güvenli fallback. */
export function statusToken(
	map: Record<string, StatusToken>,
	status: string
): StatusToken {
	return map[status] ?? {
		label: status,
		badgeClass: 'bg-gray-100 text-gray-700 border border-gray-300 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-600',
		dotClass: 'bg-gray-500'
	};
}

/** Öncelik görselleri (MASTER PLAN §22). */
export const PRIORITY: Record<string, StatusToken> = {
	LOW: {
		label: 'Düşük',
		badgeClass: 'bg-gray-100 text-gray-600 border border-gray-300 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-600',
		dotClass: 'bg-gray-400'
	},
	NORMAL: {
		label: 'Normal',
		badgeClass: 'bg-blue-50 text-blue-700 border border-blue-200 dark:bg-blue-900/30 dark:text-blue-300 dark:border-blue-800',
		dotClass: 'bg-in-progress'
	},
	HIGH: {
		label: 'Yüksek',
		badgeClass: 'bg-amber-50 text-amber-700 border border-amber-200 dark:bg-amber-900/30 dark:text-amber-300 dark:border-amber-800',
		dotClass: 'bg-status-ready'
	},
	URGENT: {
		label: 'Acil',
		badgeClass: 'bg-red-50 text-red-700 border border-red-200 dark:bg-red-900/30 dark:text-red-300 dark:border-red-800',
		dotClass: 'bg-blocked'
	}
};
