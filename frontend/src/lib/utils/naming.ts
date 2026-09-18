/**
 * Bulk/klon isim üretimi — backend (section_service) ile aynı mantık.
 * UI'da önizleme ve varsayılan format doldurma için kullanılır.
 */

/** `{n}` / `{n:2}` yer tutucularını sıra numarasıyla değiştirir. */
export function renderFormat(format: string, n: number): string {
	let out = format;
	for (const [pad, placeholder] of [
		[2, '{n:2}'],
		[3, '{n:3}']
	] as const) {
		out = out.split(placeholder).join(String(n).padStart(pad, '0'));
	}
	return out.split('{n}').join(String(n));
}

/** Klon adı: öndeki sayıyı artırır ("1. Kat" + 1 → "2. Kat"), yoksa kopya eki. */
export function deriveCloneName(source: string, k: number): string {
	const match = /^\d+/.exec(source);
	if (!match) return `${source} (kopya ${k})`;
	const digits = match[0];
	const num = parseInt(digits, 10);
	return String(num + k).padStart(digits.length, '0') + source.slice(digits.length);
}

/** Bölüm tipi etiketleri — görünteleme amaçlı, yeni tipler serbest tanımlanabilir. */
const TYPE_LABELS: Record<string, string> = {
	BLOCK: 'Blok',
	FLOOR: 'Kat',
	APARTMENT: 'Daire',
	ROOM: 'Oda',
	VILLA: 'Villa'
};

export function sectionTypeLabel(type?: string | null): string | null {
	if (!type) return null;
	return TYPE_LABELS[type] ?? type;
}
