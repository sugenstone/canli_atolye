# Canlı Atölye — Üretim ve Montaj Takip Sistemi

Hiyerarşik proje yapılarındaki iş kalemlerinin, tanımlanabilir süreçler boyunca **gerçek zamanlı** olarak takip edildiği genel amaçlı üretim ve saha operasyon sistemi.

İlk kullanım senaryosu: 15 kat × 10 daire içeren bir site projesinde her daireye mutfak tezgahı üretimi (Kesim → İmalat → Nakliye → Montaj) takibi. Ancak sistem hiçbir üretim sürecine özel değildir — süreçler, bölüm yapıları ve özellikler kod değişikliği olmadan kullanıcı tarafından tanımlanır.

## Öne Çıkanlar

- **Sınırsız iç içe bölüm yapısı** — Blok → Kat → Daire → Oda...; toplu üretim ("Daire 1..10") ve alt ağaç çoğaltma ("katı ×14")
- **Dinamik iş kalemi tipleri ve özellikler** — "Metraj (sayı, m)", "Malzeme (seçim listesi)" gibi workspace seviyesinde tanımlanır, tipli doğrulama ile kaydedilir
- **Süreç motoru** — şablon/grup/bağımlılık tanımları (lineer veya elmas), state machine (PENDING→READY→IN_PROGRESS→COMPLETED + PAUSED/BLOCKED), optimistic locking, append-only olay geçmişi
- **Canlı yönetim** — kat/daire matrisi (renk+ikon+tooltip), kanban üretim akışı, aktivite timeline, **SSE ile sayfa yenilemeden güncellenen dashboard**
- **TV Modu** — atölye monitörü için tam ekran, koyu tema, canlı saat ve ilerleme çubukları (`/tv/{projeId}`)
- **Çalışan ekranı** — mobil öncelikli "Benim İşlerim": dev Başlat/Duraklat/Bitir butonları, canlı süre sayacı, sorun bildirme (neden kataloğu ile)
- **Planlama** — plan tarihleri, gecikme (LATE) tespiti, toplu plan değişimi, aktif/bekleme/bloke süre kırılımı
- **Raporlar** — süreç performansı (ort. aktif/bekleme süresi), takım performansı, darboğaz analizi

## Teknoloji Yığını

| Katman | Teknoloji |
|---|---|
| Frontend | SvelteKit + TypeScript + Tailwind CSS + Flowbite Svelte |
| Backend | Rust + Axum + Tokio + Serde |
| Veritabanı | SQLx — geliştirme: SQLite, staging/prod: PostgreSQL |
| Realtime | Server-Sent Events (SSE) |
| Kimlik doğrulama | Server-side session + HttpOnly cookie |
| Kimlik doğrulama deposu | PostgreSQL (plan); SQLite (dev) |

## Çalıştırma

```bash
# Backend (http://127.0.0.1:8080)
cd backend
cp .env .env.local  # gerekirse düzenleyin
cargo run           # migration + seed otomatik (ilk admin: .env değerleri)

# Frontend (http://localhost:5173, API'yi 8080'e proxy'ler)
cd frontend
npm install
npm run dev
```

Giriş: `.env` içindeki `SEED_ADMIN_EMAIL` / `SEED_ADMIN_PASSWORD` (varsayılan `admin@canliatolye.local` / `admin123` — **produksiyonda değiştirin**).

## Mimari

Katmanlı yapı: `api` (ince handler'lar) → `application` (service) → `domain` (saf kurallar: state machine, süre hesaplayıcı, RBAC) → `infrastructure` (repository, storage, SSE hub).

Ayrıntılı tasarım: [`docs/architecture.md`](docs/architecture.md) · Ürün gereksinimleri: `MASTER-PLAN.md`

## Testler

```bash
cd backend && cargo test   # 75 test: state machine matrisi, bağımlılık çözümleyici,
                           # süre hesaplayıcı, RBAC, akışlar (auth/sections/work-items/
                           # process/operations/insights/planning/reports/my-work)
```

## Lisans

Özel proje — tüm hakları saklıdır.
