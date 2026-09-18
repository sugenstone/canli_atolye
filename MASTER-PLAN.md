# Üretim ve Montaj Takip Sistemi — MASTER PLAN

## 1. Proje Özeti

Bu proje; çok sayıda bağımsız bölüm, daire, villa, blok veya benzeri fiziksel birim içeren projelerde iş kalemlerinin üretim ve saha süreçlerini takip etmek için geliştirilecek **responsive web tabanlı üretim operasyon yönetim sistemidir**.

İlk kullanım senaryosu:

- 15 katlı bina
- Her katta 10 daire
- Toplam 150 daire
- Her dairede mutfak tezgahı ve ileride eklenebilecek diğer iş kalemleri
- Temel süreçler:
  - Kesim
  - İmalat
  - Nakliye
  - Montaj

Sistem yalnızca mutfak tezgahına özel tasarlanmamalıdır. Yapı tamamen dinamik olmalı ve ileride farklı üretim/hizmet süreçlerini de destekleyebilmelidir.

---

# 2. Ana Hedef

Sistem yöneticisinin monitör başından aşağıdaki sorulara saniyeler içinde cevap verebilmesi gerekir:

- Hangi işler tamamlandı?
- Hangi işler şu anda devam ediyor?
- Hangi işler bekliyor?
- Hangi işler bloke oldu?
- Kim hangi iş üzerinde çalışıyor?
- Bir iş ne zaman başladı?
- Ne kadar sürdü?
- Hangi aşamada darboğaz oluştu?
- Bugün kaç iş tamamlandı?
- Hangi işler gecikiyor?
- Hangi ekip ne kadar iş yaptı?
- Bir dairenin tüm süreç geçmişi nedir?

Çalışan tarafındaki temel hedef ise mümkün olduğunca az kullanıcı etkileşimidir.

Çalışan mümkünse sadece:

- Başlat
- Duraklat
- Devam Et
- Bitir
- Sorun Bildir

işlemlerini yapmalıdır.

---

# 3. Temel Tasarım İlkeleri

## 3.1. Sistem dinamik olmalı

Şu yapı kesinlikle hard-code edilmemeli:

```text
Kat → Daire → Kesim → İmalat → Nakliye → Montaj
```

Bunun yerine:

```text
Workspace
  └── Proje
       └── Bölüm
            └── Bölüm
                 └── Bölüm
                      └── İş Kalemi
                           └── Süreç Çalıştırmaları
```

Bölümler sınırsız seviyede iç içe oluşturulabilmelidir.

Örnek:

```text
Gardenia
├── A Blok
│   ├── 1. Kat
│   │   ├── Daire 101
│   │   │   ├── Mutfak Tezgahı
│   │   │   └── Banyo Tezgahı
│   │   └── Daire 102
│   └── 2. Kat
└── B Blok
```

Başka proje:

```text
Villa Projesi
├── Villa 1
│   ├── Zemin Kat
│   ├── 1. Kat
│   └── Teras
└── Villa 2
```

---

# 4. Sistem Kapsamı

## 4.1. Dahil

- Responsive web uygulaması
- Masaüstü yönetici arayüzü
- Mobil uyumlu çalışan arayüzü
- Workspace yönetimi
- Proje yönetimi
- Sonsuz iç içe bölüm yapısı
- İş kalemi tanımlama
- Dinamik özellik sistemi
- Süreç grupları
- Süreç şablonları
- Süreç bağımlılıkları
- Kullanıcı ve ekip atama
- Başlat / duraklat / devam et / bitir
- Bloke / sorun bildirimi
- Fotoğraf ve dosya ekleme
- İşlem geçmişi
- Canlı dashboard
- Kat/daire veya bölüm matrisi
- Üretim akışı görünümü
- Gecikme takibi
- Planlanan / gerçekleşen zaman
- Yetkilendirme
- Toplu işlemler
- Revizyon / yeniden işleme
- Raporlama
- Temel performans analitiği

## 4.2. Şimdilik dahil değil

- Offline çalışma
- Offline senkronizasyon
- QR kod
- Barkod
- Native mobil uygulama
- IoT / makine entegrasyonu
- ERP entegrasyonu
- Muhasebe
- Stok yönetimi
- Satın alma modülü
- Personel bordro sistemi

Bu özellikler ileride eklenebilmelidir fakat MVP mimarisini gereksiz yere karmaşıklaştırmamalıdır.

---

# 5. Kullanıcı Rolleri

Sistem role-based access control desteklemelidir.

İlk roller:

## ADMIN

Tüm sisteme erişebilir.

Yetkiler:

- Workspace yönetimi
- Proje oluşturma
- Kullanıcı oluşturma
- Takım oluşturma
- Süreç şablonu oluşturma
- Süreç grubu oluşturma
- İş kalemi oluşturma
- Atama yapma
- Her süreci görüntüleme ve değiştirme
- Rapor görüntüleme

## PROJECT_MANAGER

Belirlenen projelerde:

- Tüm bölümleri görür
- Süreç durumlarını görür
- Atama yapabilir
- Bloke kaldırabilir
- Plan tarihlerini değiştirebilir
- Toplu işlem yapabilir
- Raporları görüntüleyebilir

## TEAM_LEADER

Kendi takımına ait işleri yönetir.

- Takım işlerini görür
- Süreç başlatabilir
- Bitirebilir
- Sorun bildirebilir
- Takım üyelerini görevlendirebilir

## WORKER

Sadece yetkili olduğu işleri görür.

Temel işlemleri:

- Başlat
- Duraklat
- Devam Et
- Bitir
- Sorun bildir
- Not ekle
- Fotoğraf ekle

## VIEWER

Salt okunur erişim.

---

# 6. Ana Domain Modeli

## Workspace

Üst seviye çalışma alanı.

Alanlar:

```text
id
name
slug
description
createdAt
updatedAt
```

---

## Project

Bir işi/projeyi temsil eder.

```text
id
workspaceId
name
code
description
status
plannedStartDate
plannedEndDate
actualStartDate
actualEndDate
createdAt
updatedAt
```

Durumlar:

```text
DRAFT
ACTIVE
PAUSED
COMPLETED
CANCELLED
ARCHIVED
```

---

## Section

Proje içerisindeki sınırsız hiyerarşik yapıdır.

Örnek:

```text
A Blok
  └── 8. Kat
       └── Daire 83
```

Alanlar:

```text
id
projectId
parentId nullable
name
code
type nullable
sortOrder
metadata
createdAt
updatedAt
```

`parentId` recursive yapıyı sağlar.

Section için sabit "kat", "daire", "blok" kolonları oluşturulmamalıdır.

`type` sadece görsel/filtreleme amacıyla kullanılabilir.

Örnek:

```text
BLOCK
FLOOR
APARTMENT
ROOM
VILLA
CUSTOM
```

Yeni type tanımlanabilmesi tercih edilir.

---

# 7. İş Kalemi Modeli

## WorkItem

Takibi yapılan esas iştir.

Örnek:

```text
Mutfak Tezgahı
Banyo Tezgahı
Ada
TV Ünitesi
Duvar Kaplama
Masa
```

Alanlar:

```text
id
projectId
sectionId
workItemTypeId
name
code
status
priority
plannedStartDate
plannedEndDate
createdAt
updatedAt
```

---

# 8. Dinamik Özellik Sistemi

İş kalemlerine dinamik özellikler eklenebilmelidir.

Örnek:

```text
Malzeme: Lamar Moon White
Metraj: 5.82
Kalınlık: 12 mm
Eviye Tipi: Alttan
Ocak Tipi: Ankastre
Renk: Beyaz
```

## PropertyDefinition

```text
id
workspaceId
name
key
dataType
unit nullable
required
options nullable
```

Data type:

```text
TEXT
LONG_TEXT
NUMBER
DECIMAL
BOOLEAN
DATE
DATETIME
SELECT
MULTI_SELECT
```

## PropertyValue

```text
id
workItemId
propertyDefinitionId
value
```

Mümkünse typed value kolonları veya güvenli JSON yapısı kullanılmalıdır.

---

# 9. Süreç Şablonları

Kesim, imalat, nakliye ve montaj uygulama koduna gömülmemelidir.

## ProcessTemplate

Örnek:

```text
Kesim
İmalat
Kalite Kontrol
Paketleme
Nakliye
Montaj
```

Alanlar:

```text
id
workspaceId
name
code
description
defaultDuration nullable
color nullable
icon nullable
active
```

---

# 10. Süreç Grupları

Bir iş kalemine uygulanabilecek süreç dizisidir.

Örnek:

## Standart Tezgah Süreci

```text
1. Kesim
2. İmalat
3. Nakliye
4. Montaj
```

Başka grup:

## Gelişmiş Tezgah Süreci

```text
1. Ölçü
2. Çizim
3. Kesim
4. CNC
5. İmalat
6. Kalite Kontrol
7. Paketleme
8. Nakliye
9. Montaj
```

## ProcessGroup

```text
id
workspaceId
name
description
active
```

## ProcessGroupStep

```text
id
processGroupId
processTemplateId
sortOrder
required
```

---

# 11. Süreç Bağımlılıkları

Süreçler arası bağımlılık desteklenmelidir.

Örneğin:

```text
İmalat başlamak için:
Kesim COMPLETED olmalı.
```

Ancak sadece lineer yapı varsayılmamalıdır.

İleride:

```text
Montaj
  requires:
  - Nakliye tamamlandı
  - Kalite kontrol tamamlandı
```

olabilir.

## ProcessDependency

```text
id
processGroupStepId
dependsOnProcessGroupStepId
requiredStatus
```

İlk sürümde çoğu süreç lineer oluşturulabilir fakat veri modeli çoklu bağımlılığı desteklemelidir.

---

# 12. Process Execution

Sistemin en önemli entity'lerinden biridir.

Bir WorkItem'a ProcessGroup uygulandığında ilgili adımlar için ProcessExecution kayıtları oluşturulur.

Örnek:

```text
Daire 83
Mutfak Tezgahı

Kesim          COMPLETED
İmalat         IN_PROGRESS
Nakliye        PENDING
Montaj         PENDING
```

## ProcessExecution

```text
id
workItemId
processTemplateId
processGroupStepId

status

assignedUserId nullable
assignedTeamId nullable

plannedStartAt nullable
plannedEndAt nullable

readyAt nullable
startedAt nullable
completedAt nullable

createdAt
updatedAt
```

---

# 13. Süreç Durumları

Ana durumlar:

```text
PENDING
READY
IN_PROGRESS
PAUSED
BLOCKED
COMPLETED
CANCELLED
```

## PENDING

Ön koşulları henüz tamamlanmamış.

## READY

Başlamak için gereken şartlar sağlanmış.

## IN_PROGRESS

Çalışma aktif.

## PAUSED

Çalışma geçici olarak durdurulmuş.

## BLOCKED

Harici bir problem nedeniyle devam edemiyor.

## COMPLETED

Tamamlandı.

## CANCELLED

İptal edildi.

---

# 14. Durum Geçiş Kuralları

Normal akış:

```text
PENDING
   ↓
READY
   ↓
IN_PROGRESS
   ↓
COMPLETED
```

Alternatif:

```text
IN_PROGRESS
   ↓
PAUSED
   ↓
IN_PROGRESS
```

veya:

```text
READY
   ↓
BLOCKED
   ↓
READY
```

veya:

```text
IN_PROGRESS
   ↓
BLOCKED
   ↓
IN_PROGRESS
```

Geçiş kuralları backend tarafından doğrulanmalıdır.

Frontend yalnızca UI değildir; business rule backend'de korunmalıdır.

---

# 15. Event / Audit Sistemi

Sadece ProcessExecution.status değiştirilmemelidir.

Her önemli işlem ayrıca event olarak kaydedilmelidir.

## ProcessEvent

```text
id
processExecutionId
eventType
previousStatus nullable
newStatus nullable
userId
teamId nullable
timestamp
note nullable
metadata nullable
```

Event tipleri:

```text
CREATED
READY
ASSIGNED
STARTED
PAUSED
RESUMED
BLOCKED
UNBLOCKED
COMPLETED
REOPENED
CANCELLED
NOTE_ADDED
FILE_ADDED
ASSIGNEE_CHANGED
PLANNED_DATE_CHANGED
```

Bu kayıtlar normal kullanıcılar tarafından silinememelidir.

Amaç:

```text
09:13 READY
09:17 Ahmet STARTED
10:04 PAUSED
      Sebep: Testere değişimi
10:26 RESUMED
11:02 COMPLETED
```

gibi geçmiş oluşturabilmektir.

---

# 16. Gerçek Çalışma Süresi

Elapsed time hesaplaması:

```text
Toplam aktif süre =
STARTED → PAUSED
+
RESUMED → PAUSED
+
RESUMED → COMPLETED
```

PAUSED ve BLOCKED süreleri aktif çalışma süresine dahil edilmemelidir.

Ayrıca aşağıdaki süreler ayrı hesaplanmalıdır:

```text
activeDuration
pausedDuration
blockedDuration
totalLeadTime
waitingDuration
```

Bu veriler performans analizi için kullanılacaktır.

---

# 17. Atama Sistemi

Her ProcessExecution:

```text
assignedUserId
```

veya

```text
assignedTeamId
```

ile atanabilir.

Aynı anda ikisinin dolu olması zorunlu değildir.

İleride çoklu kullanıcı gerekiyorsa participant tablosu eklenebilir.

## Team

```text
id
workspaceId
name
description
```

## TeamMember

```text
teamId
userId
role
```

---

# 18. Bloke / Sorun Sistemi

ProcessExecution bloke edilebilir.

## BlockReason

Admin tarafından yönetilebilir.

Örnek:

```text
Malzeme eksik
Ölçü hatası
Dolap hazır değil
Taş kırıldı
Makine arızası
Nakliye bekleniyor
Şantiye hazır değil
Müşteri kaynaklı
Diğer
```

## ProcessBlock

```text
id
processExecutionId
reasonId
description
createdBy
createdAt
resolvedBy nullable
resolvedAt nullable
resolutionNote nullable
```

BLOCKED olduğunda kullanıcıdan neden istenmelidir.

---

# 19. Dosya ve Fotoğraf Sistemi

Aşağıdakilere dosya bağlanabilmelidir:

- Project
- Section
- WorkItem
- ProcessExecution
- ProcessEvent

Örnek dosyalar:

```text
Fotoğraf
PDF
DWG
DXF
Excel
JPG
PNG
WebP
```

## Attachment

```text
id
workspaceId
entityType
entityId
fileName
fileUrl
mimeType
size
uploadedBy
createdAt
```

Dosya sistemi storage abstraction kullanmalıdır.

Local filesystem'e sıkı bağlı geliştirilmemelidir.

Üretimde S3-compatible object storage kullanılabilmelidir.

---

# 20. Revizyon / Yeniden İşleme

Tamamlanmış bir süreç tekrar açıldığında geçmiş veri değiştirilmemelidir.

Örneğin:

```text
Montaj COMPLETED
↓
Ürün hasarlı bulundu
↓
Revizyon açıldı
```

İlk kayıt korunmalıdır.

Mümkün yaklaşım:

```text
ProcessExecution
revisionNo: 0

ProcessExecution
revisionNo: 1
parentExecutionId: originalId
```

Alternatif olarak Rework entity kullanılabilir.

Ana prensip:

> Tarihçe hiçbir zaman yeniden yazılmamalıdır.

---

# 21. Planlanan ve Gerçek Zamanlar

Her süreçte:

```text
plannedStartAt
plannedEndAt

startedAt
completedAt
```

ayrı tutulmalıdır.

Dashboard şu hesapları yapabilmelidir:

```text
Planlanan bitiş: 14:00
Gerçek bitiş: 15:23

Gecikme:
1 saat 23 dakika
```

---

# 22. Öncelik

WorkItem veya ProcessExecution:

```text
LOW
NORMAL
HIGH
URGENT
```

önceliğine sahip olabilmelidir.

Yönetici gerektiğinde sırayı değiştirebilmelidir.

---

# 23. Toplu İşlemler

150+ iş üzerinde tek tek işlem yapmak zorunda kalınmamalıdır.

Yönetici şu işlemleri toplu yapabilmelidir:

- Sorumlu kullanıcı ata
- Takım ata
- Plan tarihi değiştir
- Öncelik değiştir
- Süreç grubu ata
- Bölüm kopyala
- İş kalemi kopyala
- Durum filtrele
- Excel dışa aktar

Örnek:

```text
8. Kat
10 daire seçildi

→ Nakliye Ekibi 2'ye ata
```

---

# 24. Proje Oluşturma ve Kopyalama

Proje kurulumunu hızlandırmak için Section ağaçları kopyalanabilmelidir.

Örneğin:

```text
1. Kat
├── Daire 01
├── Daire 02
...
└── Daire 10
```

oluşturulduktan sonra:

```text
Katı çoğalt × 14
```

yapılabilmelidir.

İsim ve kod üretimi desteklenmelidir.

Örnek:

```text
Kat 01
Kat 02
...
Kat 15
```

Daire oluşturma aracı:

```text
Başlangıç: 1
Adet: 10
Format: Daire {n}
```

gibi bulk generator içerebilir.

---

# 25. Dashboard

Ana yönetici dashboardunda:

## KPI kartları

```text
Toplam İş
Tamamlanan
Devam Eden
Bekleyen
Bloke
Geciken
Bugün Tamamlanan
```

## Süreç ilerlemeleri

```text
Kesim      112 / 150
İmalat      96 / 150
Nakliye     74 / 150
Montaj      61 / 150
```

## Canlı çalışan işler

```text
K08 / D083
İmalat
İmalat Ekibi 1
01:27:42
```

## Dikkat gerekenler

```text
BLOCKED
LATE
PAUSED_TOO_LONG
WAITING_TOO_LONG
```

## Darboğaz

Örnek:

```text
KESİM READY          4
İMALAT READY        27
NAKLİYE READY        8
MONTAJ READY         3
```

Buradan imalatta yığılma olduğu anlaşılabilir.

---

# 26. Canlı Dashboard

Dashboard sayfası refresh gerektirmemelidir.

Bir çalışan:

```text
KESİMİ BAŞLAT
```

dediğinde yönetici ekranında birkaç saniye içinde:

```text
Daire 83
Kesim → IN_PROGRESS
```

görünmelidir.

Tercih edilen teknolojiler:

```text
WebSocket
veya
Server-Sent Events
```

Mimari sade tutulacaksa SSE yeterli olabilir.

Çift yönlü gerçek zamanlı özellikler çoğalırsa WebSocket tercih edilebilir.

---

# 27. Kat / Daire / Bölüm Matrisi

Sistemin en güçlü ekranlarından biri olacaktır.

Section ağacındaki belirli seviyeler kullanıcı tarafından satır ve sütun olarak seçilebilmelidir.

İlk proje için:

```text
           D01 D02 D03 D04 D05 D06 D07 D08 D09 D10

15. Kat
14. Kat
13. Kat
...
1. Kat
```

Her hücrede özet durum gösterilebilir.

Renk önerisi:

```text
Gri       Bekliyor
Sarı      Hazır
Mavi      Devam Ediyor
Turuncu   Duraklatılmış
Kırmızı   Bloke
Yeşil     Tamamlandı
```

Sadece renge güvenilmemelidir.

Renk + ikon + tooltip kullanılmalıdır.

Accessibility açısından renk körlüğü dikkate alınmalıdır.

---

# 28. Hücre Detay Paneli

Matriste hücreye tıklandığında yeni sayfa açmak yerine sağ drawer açılmalıdır.

Örnek:

```text
KAT 08 / DAİRE 083

Mutfak Tezgahı
Lamar Moon White
5.82 mtül

✓ Kesim
  Ahmet
  08:42 → 09:57

● İmalat
  İmalat Ekibi 1
  Başlangıç: 10:18
  01:27:32

○ Nakliye
  Bekliyor

○ Montaj
  Bekliyor
```

Drawer içinde:

- Genel bilgi
- Süreç listesi
- Fotoğraflar
- Notlar
- Geçmiş
- Atamalar

sekmeleri olabilir.

---

# 29. Üretim Akışı Ekranı

Operasyon yöneticisi için Kanban benzeri ekran.

Kolonlar:

```text
Kesim
İmalat
Nakliye
Montaj
```

Her kolon içerisinde:

```text
READY
IN_PROGRESS
BLOCKED
```

kartları görüntülenebilir.

Ancak kullanıcıların kartları keyfi olarak sürükleyerek status değiştirmesi önerilmez.

Durum değişiklikleri business rule'a göre yapılmalıdır.

Drag/drop yalnızca sıralama veya atama amaçlı kullanılabilir.

---

# 30. Canlı Fabrika / TV Modu

Tam ekran okunabilir dashboard.

Amaç:

Atölyedeki monitör veya TV'de sürekli açık kalabilmesi.

Gösterilecekler:

```text
Proje ilerleme yüzdesi
Kesim ilerleme
İmalat ilerleme
Nakliye ilerleme
Montaj ilerleme

Şu anda yapılan işler

Bloke işler
Geciken işler
Bugün tamamlanan işler
```

Bu modda gereksiz navigasyon gizlenmelidir.

---

# 31. Çalışan Arayüzü

Mobil öncelikli responsive tasarım.

Ana sayfa:

```text
Benim İşlerim

Devam Eden
Hazır
Bekleyen
Tamamlanan
```

İş kartı:

```text
KAT 08 / DAİRE 083

Mutfak Tezgahı

KESİM

[ BAŞLAT ]
```

Başladıktan sonra:

```text
KESİM DEVAM EDİYOR

01:24:37

[ DURAKLAT ]
[ SORUN BİLDİR ]

[ İŞİ BİTİR ]
```

Butonlar:

- Büyük
- Yüksek kontrastlı
- Kolay basılabilir
- Minimum metinli

olmalıdır.

---

# 32. Responsive Tasarım

Uygulama web tabanlı olacaktır.

Destek:

```text
Desktop
Laptop
Tablet
Mobil
```

Yönetici dashboardu desktop öncelikli olabilir.

Çalışan arayüzü mobil öncelikli olmalıdır.

Mobilde:

- Sidebar drawer'a dönüşmeli
- Tablolar card/list görünümüne dönüşebilmeli
- Sticky action bar kullanılmalı
- Kritik butonlar ekran altında erişilebilir olmalı

---

# 33. Arama ve Filtreleme

Global arama desteklenmelidir.

Örnek:

```text
Daire 83
83
Moon White
Ahmet
```

Filtreler:

```text
Proje
Bölüm
İş kalemi
Süreç
Durum
Sorumlu
Takım
Öncelik
Gecikme
Tarih
```

Filtreler URL query parametrelerinde tutulabilir.

Örnek:

```text
/project/123/processes?status=BLOCKED&team=4
```

Bu sayede ekranlar bookmark/share yapılabilir.

---

# 34. Bildirim Sistemi

İlk sürümde uygulama içi notification yeterlidir.

Örnek kurallar:

```text
Process BLOCKED oldu
Planlanan bitiş geçti
READY durumda 6 saat bekledi
PAUSED durumda 4 saat kaldı
Sorumlu değişti
```

## Notification

```text
id
userId
type
title
message
entityType
entityId
readAt
createdAt
```

E-posta/SMS/WhatsApp entegrasyonu daha sonra eklenebilir.

---

# 35. Gecikme Kuralları

Sistem gecikmeyi otomatik belirlemelidir.

Örneğin:

```text
status != COMPLETED
AND
now > plannedEndAt

=> LATE
```

Ayrıca:

```text
READY for > threshold
```

durumları ayrı bir uyarı olabilir.

---

# 36. Raporlama

İlk raporlar:

## Proje Özeti

```text
Toplam iş kalemi
Tamamlanan
Devam eden
Bloke
Geciken
```

## Süreç Performansı

```text
Kesim
Ortalama aktif süre
Ortalama bekleme süresi
Toplam tamamlanan
```

## Personel / Takım

```text
Tamamlanan işlem sayısı
Toplam aktif süre
Ortalama işlem süresi
```

Bu veriler personeli cezalandırma veya puanlama amacıyla doğrudan yorumlanmamalıdır.

İşlerin zorluk dereceleri farklı olabilir.

---

# 37. Darboğaz Analizi

Dashboard süreçlerdeki birikmeyi göstermelidir.

Örnek:

```text
KESİM
READY: 4
IN_PROGRESS: 2

İMALAT
READY: 31
IN_PROGRESS: 5

NAKLİYE
READY: 3
```

Bu durumda sistem:

```text
Olası darboğaz: İMALAT
```

gösterebilir.

İlk sürümde basit eşik tabanlı analiz yeterlidir.

---

# 38. Aktivite Akışı

Proje seviyesinde timeline:

```text
11:48 Mehmet — D083 Kesimi tamamladı
11:45 Montaj Ekibi 2 — D044 montajını başlattı
11:41 Hasan — D072 imalatını bloke etti
11:35 Yönetici — D081 sorumlusunu değiştirdi
```

Filtrelenebilir olmalıdır.

---

# 39. Optimistic UI

Başlat/Bitir gibi aksiyonlarda UI hızlı tepki vermelidir.

Ancak server işlemi başarısız olursa kullanıcıya açık hata gösterilmelidir.

Kritik business state server source-of-truth olmalıdır.

---

# 40. Concurrency

Aynı ProcessExecution iki farklı kullanıcı tarafından aynı anda başlatılmaya çalışılabilir.

Backend bu durumu engellemelidir.

Öneri:

```text
optimistic locking
version column
veya
transaction + row lock
```

Örnek:

```text
process_execution.version
```

update sırasında kontrol edilmelidir.

---

# 41. Transaction Kullanımı

Örneğin bir süreç tamamlanınca:

1. ProcessExecution COMPLETED
2. ProcessEvent COMPLETED
3. Sonraki süreç dependency kontrolü
4. Sonraki süreç READY olabilir
5. Notification oluşabilir

Bu işlemler mümkünse tek transaction içerisinde yapılmalıdır.

---

# 42. Soft Delete

Önemli domain kayıtları doğrudan silinmemelidir.

Tercih:

```text
deletedAt
archivedAt
```

Özellikle:

```text
Project
Section
WorkItem
ProcessExecution
ProcessEvent
```

için geçmiş korunmalıdır.

ProcessEvent mümkünse immutable olmalıdır.

---

# 43. API Tasarımı

REST tercih edilirse kaynak bazlı yapı kullanılmalıdır.

Örnek:

```text
GET    /api/projects
POST   /api/projects

GET    /api/projects/:id
PATCH  /api/projects/:id

GET    /api/projects/:id/sections
POST   /api/projects/:id/sections

GET    /api/work-items/:id
PATCH  /api/work-items/:id

GET    /api/process-executions/:id

POST   /api/process-executions/:id/start
POST   /api/process-executions/:id/pause
POST   /api/process-executions/:id/resume
POST   /api/process-executions/:id/block
POST   /api/process-executions/:id/unblock
POST   /api/process-executions/:id/complete
```

Status değişiklikleri:

```text
PATCH status=COMPLETED
```

yerine action endpoint ile yapılması önerilir.

Çünkü süreç geçişlerinde business logic vardır.

---

# 44. Backend Service Katmanı

Controller doğrudan DB işlemi yapmamalıdır.

Önerilen yapı:

```text
Controller / Route
    ↓
Application Service
    ↓
Domain Rules
    ↓
Repository / ORM
```

Örnek:

```text
ProcessExecutionService.start()
ProcessExecutionService.pause()
ProcessExecutionService.resume()
ProcessExecutionService.block()
ProcessExecutionService.complete()
```

---

# 45. Süreç Motoru

Merkezi bir Process Engine / Workflow Service bulunmalıdır.

Sorumlulukları:

```text
canStart()
canPause()
canResume()
canComplete()
canBlock()

checkDependencies()
markReadyProcesses()
calculateDurations()
```

Business logic UI'a dağılmamalıdır.

---

# 46. Önerilen Frontend Mimari Yapısı

Framework bağımsız prensip:

```text
src/
├── app/
├── features/
│   ├── projects/
│   ├── sections/
│   ├── work-items/
│   ├── processes/
│   ├── teams/
│   ├── users/
│   ├── dashboard/
│   └── reports/
│
├── components/
├── hooks/
├── services/
├── lib/
├── types/
└── utils/
```

Feature-based yapı tercih edilmelidir.

---

# 47. Önerilen Sayfalar

```text
/login

/dashboard

/projects
/projects/:projectId

/projects/:projectId/dashboard
/projects/:projectId/tree
/projects/:projectId/matrix
/projects/:projectId/flow
/projects/:projectId/activity
/projects/:projectId/reports

/work-items/:id

/my-work

/admin/users
/admin/teams
/admin/process-templates
/admin/process-groups
/admin/properties
```

---

# 48. Ana Navigasyon

Desktop:

```text
Dashboard

Projeler

Benim İşlerim

Üretim

Raporlar

Yönetim
```

Proje içerisine girildiğinde:

```text
Genel Bakış
Bölümler
Matris
Üretim Akışı
Aktiviteler
Raporlar
Ayarlar
```

---

# 49. UI Tasarım İlkeleri

Uygulama profesyonel üretim yazılımı hissi vermelidir.

Kaçınılacaklar:

- Gereksiz gradient
- Aşırı animasyon
- Çok renkli kartlar
- Fazla yuvarlatılmış her şey
- Dashboardu süs amaçlı grafiklerle doldurmak

Tercih:

- Temiz
- Yoğun bilgi gösterebilen
- Hızlı taranabilen
- Net tipografi
- Güçlü görsel hiyerarşi
- Tutarlı spacing
- Duruma göre renk kullanımı

---

# 50. Design System

Ortak componentler:

```text
Button
IconButton
Input
Textarea
Select
MultiSelect
DatePicker
DateTimePicker
Badge
StatusBadge
Avatar
UserPicker
TeamPicker
Drawer
Modal
Dropdown
Tabs
DataTable
FilterBar
SearchBox
Tooltip
Toast
Progress
Timeline
EmptyState
Skeleton
ConfirmDialog
```

Aynı işlev için farklı ekranlarda farklı component yapılmamalıdır.

---

# 51. Status Görsel Sistemi

Öneri:

```text
PENDING       Gray
READY         Amber
IN_PROGRESS   Blue
PAUSED        Orange
BLOCKED       Red
COMPLETED     Green
CANCELLED     Neutral/Dark Gray
```

Renk token olarak tanımlanmalıdır.

```text
--status-pending
--status-ready
--status-progress
--status-paused
--status-blocked
--status-completed
```

Hard-code renk kullanılmamalıdır.

---

# 52. Empty State

Her liste düzgün empty state göstermelidir.

Örnek:

```text
Henüz proses atanmadı.

Bu iş kalemine bir süreç grubu atayarak başlayın.

[ Süreç Grubu Ata ]
```

---

# 53. Loading UX

Tam ekran spinner yerine:

```text
Skeleton
Table row skeleton
Card skeleton
```

kullanılmalıdır.

---

# 54. Hata Yönetimi

API hata formatı standart olmalıdır.

Örnek:

```json
{
  "code": "PROCESS_NOT_READY",
  "message": "Bu işlem henüz başlatılamaz.",
  "details": {
    "missingDependencies": ["CUTTING"]
  }
}
```

UI mümkünse kullanıcıya teknik hata göstermemelidir.

---

# 55. Confirmation Kuralları

Her aksiyonda confirm istemek UX'i bozar.

Confirm yalnızca:

- İptal
- Yeniden açma
- Toplu değişiklik
- Arşivleme
- Kritik manuel status override

gibi işlemlerde kullanılmalıdır.

`Başlat` için confirm gereksizdir.

---

# 56. Yetki Kontrolü

Yetki sadece frontend'de gizlenen butonlardan ibaret olmamalıdır.

Backend her istekte authorization yapmalıdır.

Örnek:

```text
Worker
→ sadece atanmış ProcessExecution üzerinde start yapabilir
```

---

# 57. Tenant / Workspace İzolasyonu

Workspace kullanan sistemde bütün sorgular workspace sınırında çalışmalıdır.

Bir workspace'in kaydı başka workspace tarafından görülememelidir.

Bu kurala:

```text
Project
Section
WorkItem
Process
Team
User
Attachment
```

dahil edilmelidir.

---

# 58. Database Indexleri

İlk baştan düşünülmesi önerilen indexler:

```text
projectId
sectionId
workItemId
processExecution.status
assignedUserId
assignedTeamId
plannedEndAt
createdAt
```

Composite indexler:

```text
(projectId, status)

(workItemId, processGroupStepId)

(assignedUserId, status)

(assignedTeamId, status)
```

---

# 59. Audit

Aşağıdaki işlemler audit log'a yazılmalıdır:

```text
Kullanıcı oluşturuldu
Yetki değiştirildi
Proje oluşturuldu
Section silindi/arşivlendi
Süreç atandı
Sorumlu değişti
Plan tarihi değişti
Süreç manuel override edildi
```

ProcessEvent ile AdminAuditLog ayrı tutulabilir.

---

# 60. Dashboard Performansı

150 iş küçük görünse bile sistem gelecekte binlerce kayıt içerebilir.

Dashboard her render'da tüm event kayıtlarını frontend'e yüklememelidir.

Backend aggregate endpoint sunmalıdır.

Örnek:

```text
GET /api/projects/:id/dashboard-summary
```

Response:

```json
{
  "workItems": {
    "total": 150,
    "completed": 61
  },
  "processes": {
    "cutting": {
      "total": 150,
      "completed": 112
    }
  }
}
```

---

# 61. Pagination

Event, activity ve work item listelerinde cursor-based pagination tercih edilebilir.

Özellikle activity timeline büyüyecektir.

---

# 62. Cache

İlk sürümde aşırı cache mimarisinden kaçınılmalıdır.

Gerekirse:

```text
dashboard aggregates
reference data
process templates
```

cache edilebilir.

Önce ölç, sonra optimize et.

---

# 63. MVP Kapsamı

İlk sürüm aşağıdaki işleri eksiksiz yapmalıdır.

## Faz 1 — Temel Altyapı

- Authentication
- Workspace
- Users
- Teams
- Roles / permissions
- Projects

## Faz 2 — Proje Yapısı

- Recursive Sections
- Section tree
- Bulk section creation
- Section cloning

## Faz 3 — İş Kalemleri

- WorkItem
- WorkItemType
- Dynamic properties
- Bulk WorkItem creation

## Faz 4 — Süreç Motoru

- ProcessTemplate
- ProcessGroup
- ProcessGroupStep
- Dependencies
- ProcessExecution
- State machine
- ProcessEvent

## Faz 5 — Operasyon

- Assign user/team
- Start
- Pause
- Resume
- Complete
- Block
- Unblock
- Notes
- Attachments

## Faz 6 — Yönetici UX

- Dashboard
- Project dashboard
- Matrix
- Drawer detail
- Production flow
- Activity timeline

## Faz 7 — Çalışan UX

- My Work
- Mobile responsive workflow
- Large action buttons

## Faz 8 — Planlama

- plannedStartAt
- plannedEndAt
- late detection
- priorities

## Faz 9 — Reporting

- Project progress
- Process durations
- Team performance
- Bottleneck indicators

---

# 64. MVP'de Yapılmaması Gerekenler

AI ajan ilk sürümü gereksiz büyütmemelidir.

Şimdilik yapma:

```text
QR
Barcode
Offline sync
Native app
AI tahmin sistemi
ERP
Muhasebe
Stok
Satın alma
GPS
Personel takip
IoT
Makine API entegrasyonu
WhatsApp otomasyonu
```

Bunlar için veri modeli kapıyı kapatmamalıdır ancak uygulama kodu yazılmamalıdır.

---

# 65. Önerilen İlk Demo Senaryosu

Seed data:

```text
Project:
Gardenia Demo

15 kat

Her katta:
10 daire

Toplam:
150 daire
```

Her daire:

```text
Mutfak Tezgahı
```

Her WorkItem:

```text
Kesim
İmalat
Nakliye
Montaj
```

Random demo durumları:

```text
COMPLETED
IN_PROGRESS
READY
PENDING
BLOCKED
```

Demo için:

```text
6 kullanıcı
4 takım
```

oluşturulmalıdır.

---

# 66. Acceptance Criteria — Temel Süreç

Bir ProcessExecution READY ise:

```text
Worker → Başlat
```

sonucunda:

1. Status `IN_PROGRESS`
2. startedAt yazılır
3. STARTED event oluşur
4. responsible actor kaydedilir
5. dashboard canlı güncellenir

---

ProcessExecution tamamlandığında:

1. Status `COMPLETED`
2. completedAt yazılır
3. aktif süre hesaplanabilir
4. COMPLETED event oluşur
5. downstream dependency kontrol edilir
6. şartları tamamlanan sonraki süreç `READY` yapılır
7. dashboard canlı güncellenir

---

# 67. Acceptance Criteria — Bloke

Çalışan:

```text
Sorun Bildir
```

seçtiğinde:

1. Reason zorunludur
2. Açıklama opsiyoneldir
3. Fotoğraf eklenebilir
4. ProcessExecution `BLOCKED`
5. ProcessBlock oluşur
6. BLOCKED event oluşur
7. Yönetici dashboardunda uyarı görünür

---

# 68. Acceptance Criteria — History

Her process detail ekranında:

```text
Kim?
Ne yaptı?
Ne zaman?
Eski durum neydi?
Yeni durum ne?
```

görülebilmelidir.

Event geçmişi sonradan manuel olarak düzenlenmemelidir.

---

# 69. Acceptance Criteria — Matrix

150 daireli demo projede yönetici:

- Tek ekranda 15 × 10 matrisi görebilmeli
- Hücre durumunu anlayabilmeli
- Hücreye tıklayabilmeli
- Sağ panelden süreçleri görebilmeli
- Sayfa değiştirmeden başka daireye geçebilmeli

---

# 70. Acceptance Criteria — Responsive

Minimum hedef:

```text
375 px mobile
768 px tablet
1280 px desktop
1920 px large desktop
```

Temel operasyonlar tüm bu genişliklerde kullanılabilir olmalıdır.

---

# 71. AI Kodlama Ajanı İçin Kurallar

Bu doküman projenin source-of-truth ürün planıdır.

AI ajan:

1. Domain modelini keyfi değiştirmemeli.
2. Kesim/imalat/nakliye/montaj isimlerini hard-code etmemeli.
3. Section yapısını sabit blok/kat/daire kolonlarına çevirmemeli.
4. Status değişikliğini sadece frontend'de yönetmemeli.
5. ProcessEvent üretmeden process status değiştirmemeli.
6. Backend authorization kontrollerini atlamamalı.
7. Geçmiş kayıtlarını overwrite etmemeli.
8. Her ekranı aynı anda geliştirmeye çalışmamalı.
9. Önce veri modeli ve süreç motorunu sağlamlaştırmalı.
10. Modül modül ilerlemeli.
11. Her faz sonunda test çalıştırmalı.
12. Mevcut testleri bozan değişiklik yapmamalı.
13. Gereksiz dependency eklememeli.
14. Premature optimization yapmamalı.
15. MVP dışı özellikleri kendiliğinden eklememeli.

---

# 72. Geliştirme Sırası

AI ajan bu sırayı takip etmelidir:

```text
01. Project skeleton
02. Database
03. Authentication
04. Workspace
05. RBAC
06. Projects
07. Recursive Sections
08. Work Items
09. Dynamic Properties
10. Teams
11. Process Templates
12. Process Groups
13. Process Dependencies
14. Process Executions
15. State Machine
16. Process Events
17. Assignments
18. Operational Actions
19. Blocking
20. Attachments
21. Dashboard Aggregates
22. Realtime
23. Manager Dashboard
24. Matrix
25. Detail Drawer
26. Production Flow
27. Worker My Work
28. Planned vs Actual
29. Reports
30. Polish + Performance + Tests
```

---

# 73. Test Stratejisi

## Unit Tests

Özellikle:

```text
State machine
Dependency resolver
Duration calculator
Permission rules
Late detection
```

## Integration Tests

Özellikle:

```text
start process
complete process
dependency ready transition
block/unblock
team assignment
```

## E2E

Kritik senaryo:

```text
Login
→ Projeye gir
→ Daire seç
→ Kesim başlat
→ Kesim tamamla
→ İmalat READY oldu mu?
```

---

# 74. Teknik Kalite Kuralları

- Type safety tercih edilmeli
- Input validation server tarafında yapılmalı
- DB migration sistemi kullanılmalı
- Env validation olmalı
- Central error handling olmalı
- Structured logging kullanılmalı
- N+1 query problemlerinden kaçınılmalı
- Timezone işlemleri dikkatli yapılmalı
- Tarihler DB'de UTC saklanmalı
- UI'da kullanıcı timezone'una çevrilmeli

---

# 75. Güvenlik

- Password hash güvenli algoritma
- Rate limiting
- CSRF stratejisi
- XSS protection
- Secure cookies
- Authorization
- Input validation
- File upload validation
- MIME/type kontrolü
- Maksimum upload boyutu
- Audit

gereklidir.

---

# 76. Veri Yedekleme

Production ortamında:

- Otomatik DB backup
- Object storage backup/versioning
- Restore prosedürü

bulunmalıdır.

Bu özellik UI modülü olmak zorunda değildir fakat deployment planında yer almalıdır.

---

# 77. Başarı Kriteri

Proje başarılı kabul edilir eğer yönetici 150 dairelik projede:

1. Tüm projenin üretim durumunu tek dashboarddan görebiliyorsa,
2. Bir dairenin hangi aşamada olduğunu birkaç saniyede bulabiliyorsa,
3. Şu anda kimlerin ne yaptığını görebiliyorsa,
4. Bloke ve geciken işleri anında ayırt edebiliyorsa,
5. Çalışanlar mobil web ekranından birkaç dokunuşla işlem başlatıp bitirebiliyorsa,
6. Her hareketin geçmişi tutuluyorsa,
7. Yeni süreçler kod değiştirmeden oluşturulabiliyorsa,
8. Yeni proje yapıları sistem kodu değiştirmeden kurulabiliyorsa,

ana ürün hedefi karşılanmış sayılır.

---

# 78. Son Mimari Prensip

Bu proje:

> "150 dairelik mutfak tezgahını takip eden bir uygulama"

olarak geliştirilmemelidir.

Şöyle geliştirilmelidir:

> "Hiyerarşik proje yapıları içerisindeki iş kalemlerinin, tanımlanabilir süreçler boyunca gerçek zamanlı olarak takip edildiği genel amaçlı bir üretim ve saha operasyon sistemi."

İlk gerçek kullanım alanı mutfak tezgahı üretimidir.

Bu ayrım bütün mimari kararların temelidir.

---

# 79. Şimdilik Bilinçli Olarak Çıkarılan Özellikler

Bu sürüm web tabanlı ve sürekli internet bağlantısı varsayımıyla geliştirilecektir.

Bu nedenle aşağıdaki özellikler plan dışıdır:

```text
Offline çalışma
Offline queue
Service Worker ile operasyon senkronizasyonu
QR kod ile iş çağırma
Barcode tarama
```

İleride ihtiyaç oluşursa ayrı faz olarak değerlendirilmelidir.

---

# 80. İlk Kodlama Görevi

AI ajan doğrudan tüm sistemi kodlamaya başlamamalıdır.

İlk görev:

```text
1. Bu MASTER PLAN'ı analiz et.
2. Domain modelini çıkar.
3. ER diagramını yaz.
4. Database schema önerisini oluştur.
5. State machine'i tanımla.
6. RBAC izin matrisini oluştur.
7. API endpoint listesini oluştur.
8. Folder/module architecture oluştur.
9. Bunları docs/architecture.md dosyasına yaz.
10. Henüz UI geliştirmeye başlama.
```

Architecture dokümanı tamamlandıktan sonra uygulama geliştirme fazlarına geçilmelidir.

---

# 81. Sabit Teknoloji Yığını

Bu proje için teknoloji tercihleri aşağıdaki şekilde sabitlenmiştir.

## Frontend

```text
SvelteKit
TypeScript
Tailwind CSS
Flowbite Svelte
```

## Backend

```text
Rust
Axum
Tokio
Serde
SQLx
```

## Veritabanı

```text
Development: SQLite
Staging: PostgreSQL
Production: PostgreSQL
```

## Realtime

```text
İlk tercih: Server-Sent Events (SSE)
Gerekirse ileride: WebSocket
```

## Authentication

```text
Server-side session
HttpOnly cookie
Secure cookie
SameSite
CSRF koruması
```

## Dosya Saklama

```text
Development:
Local filesystem abstraction

Production:
S3-compatible object storage
```

## Deployment

```text
Docker
Reverse proxy
Rust API service
SvelteKit frontend
PostgreSQL
Object storage
```

---

# 82. SvelteKit Kullanım Kararı

Frontend yalnızca düz Svelte ile geliştirilmemelidir.

Uygulama:

```text
SvelteKit + TypeScript
```

üzerinden geliştirilmelidir.

SvelteKit aşağıdakiler için kullanılacaktır:

- Routing
- Layout
- Nested layout
- Error pages
- Auth-aware routing
- Client/server boundary
- Environment configuration
- Frontend build
- Responsive web application

Önerilen route yapısı:

```text
src/routes/

(login)/
    login/

(app)/
    dashboard/

    projects/
        [projectId]/
            dashboard/
            tree/
            matrix/
            flow/
            activity/
            reports/

    my-work/

    admin/
        users/
        teams/
        process-templates/
        process-groups/
        properties/
```

---

# 83. Frontend Feature Mimarisi

Frontend yalnızca route klasörlerinden oluşmamalıdır.

Domain/feature bazlı yapı kullanılmalıdır.

Öneri:

```text
src/
├── routes/
├── lib/
│   ├── components/
│   │   ├── ui/
│   │   ├── domain/
│   │   └── layout/
│   │
│   ├── features/
│   │   ├── projects/
│   │   ├── sections/
│   │   ├── work-items/
│   │   ├── processes/
│   │   ├── teams/
│   │   ├── users/
│   │   ├── dashboard/
│   │   └── reports/
│   │
│   ├── stores/
│   ├── services/
│   ├── types/
│   ├── utils/
│   └── config/
```

Business davranışı route componentlerine dağılmamalıdır.

---

# 84. Flowbite Svelte Kullanım Prensibi

Flowbite Svelte kullanılacaktır ancak uygulama doğrudan Flowbite API'sine bağımlı hale getirilmemelidir.

Flowbite'ın görevi:

> Genel amaçlı UI primitives sağlamak.

Örnek Flowbite kullanım alanları:

```text
Button
Modal
Drawer
Dropdown
Tabs
Tooltip
Badge
Input
Select
Checkbox
Table
Sidebar
Navbar
Toast
Pagination
Datepicker
```

Ancak aşağıdaki domain ekranları özel Svelte componentleri olarak geliştirilmelidir:

```text
Kat / Daire Matrisi
Bölüm Matrisi
Canlı Üretim Dashboardu
Üretim Akışı
Process Timeline
Canlı Fabrika Ekranı
Çalışan İş Kartları
Durum Akış Bileşenleri
Darboğaz Görselleştirmesi
```

---

# 85. UI Abstraction Kuralı

Domain componentleri doğrudan Flowbite component API'lerine sıkı şekilde bağlanmamalıdır.

Tercih edilen yapı:

```text
Flowbite
    ↓
UI Wrapper Components
    ↓
Domain Components
    ↓
Pages
```

Örnek:

```text
src/lib/components/ui/AppButton.svelte
src/lib/components/ui/AppDrawer.svelte
src/lib/components/ui/AppModal.svelte
src/lib/components/ui/AppTable.svelte
src/lib/components/ui/StatusBadge.svelte
src/lib/components/ui/ConfirmDialog.svelte
```

Domain componentleri:

```text
ProcessStartButton.svelte
ProcessCard.svelte
ProcessTimeline.svelte
WorkItemDrawer.svelte
ProjectMatrix.svelte
```

Bu prensibin amacı:

- Flowbite değişirse tüm uygulamayı yeniden yazmamak
- Tasarım sistemini tek yerde kontrol etmek
- Domain davranışını UI library'den ayırmak

---

# 86. Tailwind Kullanım Prensibi

Tailwind CSS kullanılacaktır.

Ancak:

- Her component içinde rastgele renk değerleri kullanılmamalıdır.
- Status renkleri merkezi token'lardan gelmelidir.
- Spacing tutarlı olmalıdır.
- Responsive breakpoint stratejisi ortak olmalıdır.
- Aynı görsel pattern tekrar tekrar yeniden yazılmamalıdır.

Önerilen status token mantığı:

```text
pending
ready
in-progress
paused
blocked
completed
cancelled
```

Bu değerler tek theme/config katmanından yönetilmelidir.

---

# 87. Rust Backend Kararı

Backend:

```text
Rust + Axum
```

ile geliştirilecektir.

Temel bileşenler:

```text
Axum
Tokio
Serde
SQLx
Tracing
Tower / Tower HTTP
```

Backend'in temel sorumlulukları:

- Authentication
- Authorization
- Workspace isolation
- Business rules
- State transitions
- Dependency resolution
- Audit/event oluşturma
- Transaction yönetimi
- Database access
- File metadata
- Realtime event yayınlama
- Dashboard aggregate API

---

# 88. Rust Backend Klasör Yapısı

Önerilen yapı:

```text
backend/
├── src/
│   ├── main.rs
│   ├── config/
│   ├── api/
│   │   ├── routes/
│   │   ├── handlers/
│   │   └── middleware/
│   │
│   ├── application/
│   │   ├── services/
│   │   └── dto/
│   │
│   ├── domain/
│   │   ├── entities/
│   │   ├── value_objects/
│   │   ├── errors/
│   │   └── services/
│   │
│   ├── infrastructure/
│   │   ├── db/
│   │   ├── repositories/
│   │   ├── storage/
│   │   └── realtime/
│   │
│   └── shared/
│
├── migrations/
└── Cargo.toml
```

Aşırı teorik DDD uygulanmamalıdır.

Ama business logic handler içine gömülmemelidir.

---

# 89. Axum Route Prensibi

Route handler mümkün olduğunca ince tutulmalıdır.

Önerilen akış:

```text
Axum Handler
    ↓
Application Service
    ↓
Domain Rules
    ↓
Repository
    ↓
SQLx
```

Örnek:

```text
POST /api/process-executions/:id/start
```

Handler doğrudan SQL UPDATE çalıştırmamalıdır.

Şunu çağırmalıdır:

```text
ProcessExecutionService::start(...)
```

---

# 90. Tokio Kullanımı

Tokio Rust async runtime olarak kullanılacaktır.

Kullanım alanları:

- Axum server
- Database async access
- SSE streams
- File operations
- Background olmayan request-scoped async işlemler

Bu proje başlangıçta ayrı background worker sistemi gerektirmez.

İleride gerekiyorsa job queue ayrı bir faz olarak eklenebilir.

---

# 91. Serde Kullanımı

API request/response modellerinde:

```text
serde::Serialize
serde::Deserialize
```

kullanılacaktır.

Domain entity ile dış API DTO'ları zorunlu olarak aynı struct olmak zorunda değildir.

Özellikle:

- security
- validation
- backwards compatibility

için API DTO katmanı tercih edilmelidir.

---

# 92. SQLx Kullanım Kararı

Database erişimi:

```text
SQLx
```

ile yapılacaktır.

ORM benzeri ağır abstraction yerine kontrollü SQL tercih edilmektedir.

SQLx kullanım amaçları:

- SQLite desteği
- PostgreSQL desteği
- Async query
- Transaction
- Migration
- Compile-time/checked query yaklaşımı
- Connection pool

---

# 93. SQLite Kullanım Kapsamı

SQLite yalnızca geliştirme ve küçük lokal demo ortamlarında kullanılacaktır.

Amaç:

- Hızlı kurulum
- Tek dosyalı database
- Lokal geliştirme kolaylığı
- Demo seed verisi
- AI ajan geliştirme kolaylığı

Development örneği:

```text
DATABASE_URL=sqlite://data/app.db
```

Production için SQLite kullanılmamalıdır.

---

# 94. PostgreSQL Kullanım Kapsamı

Staging ve production:

```text
PostgreSQL
```

kullanacaktır.

Neden:

- Eşzamanlı kullanıcı işlemleri
- Güvenilir transaction davranışı
- Row locking
- Daha güçlü indexing
- Production ölçeği
- Backup / replication imkanları
- Karmaşık sorgular
- Daha güçlü JSON ve tarih özellikleri

Gerçek kullanıcı kabul testlerine geçmeden önce staging PostgreSQL üzerinde çalışmalıdır.

---

# 95. SQLite → PostgreSQL Geçiş Kuralı

Uygulama ilk günden PostgreSQL'e geçiş düşünülerek yazılmalıdır.

SQLite'a özgü SQL özelliklerine gereksiz bağımlılık oluşturulmamalıdır.

Özellikle dikkat:

```text
AUTOINCREMENT
BOOLEAN davranışı
DATE/TIME davranışı
JSON fonksiyonları
UPSERT syntax
ALTER TABLE davranışı
case sensitivity
locking davranışı
```

Mümkün olduğunca iki database tarafından desteklenen SQL yaklaşımı kullanılmalıdır.

---

# 96. ID Stratejisi

Database-generated auto increment integer ID'lere sıkı bağımlılık önerilmez.

Tercih:

```text
UUID
```

veya mümkünse:

```text
UUIDv7
```

ID uygulama veya güvenilir UUID mekanizması tarafından oluşturulmalıdır.

Avantajları:

- SQLite/PostgreSQL uyumluluğu
- Dağıtık ID üretimi
- Import/export kolaylığı
- DB migration kolaylığı
- Public API'de sıralı integer ID tahminini azaltma

---

# 97. SQL Taşınabilirliği

Business logic içerisine database vendor-specific SQL gömülmemelidir.

Repository katmanında gerekiyorsa database farkları izole edilmelidir.

Öneri:

```text
repositories/
    project_repository.rs
    section_repository.rs
    work_item_repository.rs
    process_repository.rs
```

Production'a geçişte business/service katmanlarının değişmemesi hedeflenmelidir.

---

# 98. Migration Stratejisi

SQLx migration sistemi kullanılacaktır.

Örnek:

```text
migrations/
    0001_initial.sql
    0002_process_events.sql
    0003_attachments.sql
```

Migration dosyaları version control altında tutulmalıdır.

Aşağıdaki yaklaşım kullanılmamalıdır:

> Development database'i elle değiştirip migration oluşturmamak.

Her schema değişikliği migration ile yapılmalıdır.

---

# 99. Cross-Database Migration Testi

CI veya development sürecinde schema mümkünse iki database üzerinde test edilmelidir:

```text
SQLite
PostgreSQL
```

Özellikle production öncesi:

```text
Fresh PostgreSQL database
→ migrations çalıştır
→ seed yükle
→ integration tests çalıştır
```

zorunlu kabul edilmelidir.

---

# 100. Realtime Teknoloji Kararı

İlk sürüm:

```text
Server-Sent Events (SSE)
```

kullanacaktır.

Neden:

Sistemde realtime ihtiyacın büyük kısmı:

```text
Backend
   ↓
Browser
```

yönündedir.

Örnek:

```text
Çalışan:
D083 Kesimi Başlat

↓ REST POST

Rust API

↓ SSE Event

Yönetici Dashboard

D083
KESİM
IN_PROGRESS
```

---

# 101. SSE Event Türleri

Örnek eventler:

```text
process.started
process.paused
process.resumed
process.blocked
process.unblocked
process.completed
process.assigned
work_item.updated
project.updated
notification.created
```

Payload örneği:

```json
{
  "event": "process.started",
  "projectId": "uuid",
  "workItemId": "uuid",
  "processExecutionId": "uuid",
  "timestamp": "2026-09-14T10:00:00Z"
}
```

Frontend event geldiğinde ilgili query/state'i yenileyebilir.

---

# 102. WebSocket Ne Zaman Eklenmeli?

Şu anda WebSocket kullanılmamalıdır.

Aşağıdakiler eklenirse değerlendirilebilir:

- Gerçek zamanlı chat
- Operatörler arası çift yönlü realtime iletişim
- Yüksek frekanslı canlı veri
- Makine telemetry
- Çok yoğun presence sistemi

İlk sürümde SSE daha basit ve yeterlidir.

---

# 103. Frontend API Erişimi

Frontend backend API ile typed servis katmanı üzerinden konuşmalıdır.

Öneri:

```text
src/lib/services/api/
    client.ts
    projects.ts
    sections.ts
    work-items.ts
    processes.ts
    teams.ts
```

Component içerisinde doğrudan her yerde `fetch()` kullanılmamalıdır.

API error handling merkezi olmalıdır.

---

# 104. Frontend State Prensibi

Server data ile local UI state ayrılmalıdır.

Server state örnekleri:

```text
Projects
WorkItems
ProcessExecutions
Dashboard data
Activity events
```

UI state örnekleri:

```text
Drawer open
Selected row
Active tab
Filter panel
Modal state
```

Tek büyük global store kullanılmamalıdır.

Feature bazlı küçük state yapıları tercih edilmelidir.

---

# 105. SSE Frontend Entegrasyonu

Svelte tarafında merkezi realtime service oluşturulmalıdır.

Örnek:

```text
src/lib/services/realtime.ts
```

Görevi:

```text
EventSource bağlantısı
Reconnect
Event parsing
Project-level subscription
State invalidation
Error handling
```

Her component kendi EventSource bağlantısını açmamalıdır.

---

# 106. Authentication Teknik Kararı

JWT token'ı localStorage içinde saklama yaklaşımı kullanılmamalıdır.

Tercih:

```text
Server-side session
+
HttpOnly Cookie
```

Cookie ayarları production'da:

```text
HttpOnly=true
Secure=true
SameSite=Lax veya uygun strateji
```

Authentication middleware Rust backend'de bulunmalıdır.

---

# 107. Session Store

İlk geliştirmede session data database üzerinden tutulabilir.

Production için:

```text
PostgreSQL-backed session
```

yeterlidir.

Redis ilk sürüm için zorunlu değildir.

Gereksiz altyapı eklenmemelidir.

---

# 108. File Storage Abstraction

Dosya sistemi doğrudan local disk path'lerine domain içerisinde bağlanmamalıdır.

Arayüz:

```text
FileStorage
```

gibi soyutlanmalıdır.

Development implementasyonu:

```text
LocalFileStorage
```

Production:

```text
S3FileStorage
```

Domain yalnızca metadata ve storage key bilmelidir.

---

# 109. Deployment Topolojisi

Örnek production:

```text
Internet
   ↓
Reverse Proxy
   ├── Frontend
   │      SvelteKit
   │
   └── /api
          Rust Axum
             ↓
          PostgreSQL
             ↓
        Object Storage
```

Frontend/backend farklı container olabilir.

---

# 110. Docker Kararı

Development zorunlu olarak Docker içinde olmak zorunda değildir.

Ancak staging ve production deployment için Docker kullanılmalıdır.

Örnek servisler:

```text
frontend
backend
postgres
reverse-proxy
```

Object storage managed service olabilir.

---

# 111. Environment Yapısı

Örnek:

```text
.env.development
.env.test
.env.staging
.env.production
```

Rust config doğrulaması uygulama başlangıcında yapılmalıdır.

Eksik kritik ENV ile uygulama çalışmaya başlamamalıdır.

---

# 112. Logging

Rust backend structured logging kullanmalıdır.

Tercih:

```text
tracing
tracing-subscriber
```

Loglarda:

```text
request_id
user_id
workspace_id
project_id
process_execution_id
```

gibi context alanları uygun yerlerde kullanılmalıdır.

Password/token loglanmamalıdır.

---

# 113. API Versioning

İlk sürüm:

```text
/api/v1/
```

prefix kullanabilir.

Örnek:

```text
/api/v1/projects
/api/v1/process-executions/:id/start
```

İleride API değişikliklerinde avantaj sağlayacaktır.

---

# 114. Zaman Yönetimi

Database'deki timestamp değerleri UTC olarak tutulmalıdır.

Frontend:

```text
UTC
↓
kullanıcı timezone
```

şeklinde göstermelidir.

Rust tarafında timestamp handling tutarlı olmalıdır.

String bazlı manuel date işlemlerinden kaçınılmalıdır.

---

# 115. Database Transaction Kuralları

Aşağıdaki operasyonlar transaction gerektirir.

Örnek:

## Complete Process

```text
BEGIN

ProcessExecution → COMPLETED

ProcessEvent → COMPLETED

Dependency resolver çalıştır

Sonraki execution → READY

ProcessEvent → READY

Notification oluştur

COMMIT
```

Herhangi bir adım başarısız olursa:

```text
ROLLBACK
```

---

# 116. PostgreSQL'e Geçiş Öncesi Kontrol Listesi

Production/staging geçişinden önce:

- PostgreSQL migrations sıfırdan çalışıyor mu?
- Seed çalışıyor mu?
- UUID davranışı doğru mu?
- Boolean alanlar doğru mu?
- Timestamp alanları doğru mu?
- Unique constraints çalışıyor mu?
- Foreign keys doğru mu?
- Transaction testleri geçiyor mu?
- Concurrent start testi geçiyor mu?
- Dashboard aggregate sorguları hızlı mı?
- Indexler oluşturuldu mu?

---

# 117. Flowbite ile Yapılmaması Gerekenler

AI ajan:

1. Her ekranı Flowbite demo sayfasına çevirmemeli.
2. Domain UX'i Flowbite component sınırlarına göre tasarlamamalı.
3. Flowbite theme değerlerini her dosyada tekrar etmemeli.
4. Flowbite'a özel prop kullanımını domain katmanına yaymamalı.
5. Matrisi standart HTML table'a zorlamamalı.
6. Mobil worker ekranını masaüstü admin layout'un küçültülmüş hali yapmamalı.

---

# 118. Svelte Worker UX

Mobil worker ekranlarında Flowbite yalnızca primitive olarak kullanılabilir.

Ana operasyon ekranı custom tasarlanmalıdır.

Örneğin:

```text
┌────────────────────────────┐
│ D083                       │
│ Mutfak Tezgahı             │
│                            │
│ KESİM                      │
│                            │
│        01:24:37            │
│                            │
│ [ DURAKLAT ]               │
│                            │
│ [ SORUN BİLDİR ]           │
│                            │
│ [      İŞİ BİTİR       ]   │
└────────────────────────────┘
```

Mobilde ana aksiyonlar en az bir başparmakla rahat erişilebilir olmalıdır.

---

# 119. Manager UX Teknik Prensibi

Desktop manager ekranları yüksek bilgi yoğunluğunu desteklemelidir.

Responsive tasarım:

```text
Desktop:
Data density yüksek

Tablet:
Orta yoğunluk

Mobile:
Card / drawer / stacked layout
```

Mobilde desktop tabloyu yatay scroll'a terk etmek son seçenek olmalıdır.

---

# 120. İlk Teknik Mimari Görevi — Güncellenmiş

AI kodlama ajanı ilk olarak aşağıdaki dosyayı oluşturmalıdır:

```text
docs/architecture.md
```

Bu doküman şunları içermelidir:

1. SvelteKit frontend architecture
2. Flowbite wrapper strategy
3. Rust/Axum backend architecture
4. SQLx repository strategy
5. SQLite development strategy
6. PostgreSQL staging/production strategy
7. Cross-database compatibility rules
8. Entity relationship diagram
9. State machine
10. Process event model
11. RBAC permission matrix
12. REST API endpoint list
13. SSE event list
14. Transaction boundaries
15. File storage abstraction
16. Error model
17. Folder structures
18. Test strategy
19. Deployment topology

Bu doküman review edilmeden geniş çaplı UI geliştirmesine başlanmamalıdır.

---

# 121. AI Kodlama Ajanı İçin Teknik Yasaklar

AI ajan aşağıdaki kararları kendi başına değiştirmemelidir:

```text
Frontend → SvelteKit + TypeScript
CSS → Tailwind CSS
UI primitives → Flowbite Svelte
Backend → Rust + Axum
Async runtime → Tokio
Serialization → Serde
Database access → SQLx
Development DB → SQLite
Staging DB → PostgreSQL
Production DB → PostgreSQL
Realtime → SSE
Auth → HttpOnly server session
```

Ayrıca ajan:

- Prisma eklememeli.
- Drizzle eklememeli.
- Node backend oluşturmamalı.
- Supabase'e geçmemeli.
- Firebase eklememeli.
- SQLite'ı production database olarak seçmemeli.
- WebSocket'i sebepsiz yere SSE yerine getirmemeli.
- Redis'i ilk sürüme zorunlu dependency olarak eklememeli.
- Flowbite yerine kendi kararıyla başka UI kit'e geçmemeli.
- Domain state değişikliklerini frontend'e taşımamalı.

---

# 122. Son Teknik Mimari

Nihai temel mimari:

```text
┌─────────────────────────────────────────┐
│              WEB BROWSER                │
│                                         │
│  SvelteKit + TypeScript                 │
│  Tailwind + Flowbite Svelte             │
│  Custom Domain Components               │
└────────────────┬────────────────────────┘
                 │
             REST + SSE
                 │
┌────────────────▼────────────────────────┐
│              RUST API                   │
│                                         │
│  Axum                                   │
│  Tokio                                  │
│  Serde                                  │
│                                         │
│  Application Services                   │
│  Process Engine                         │
│  RBAC                                   │
│  Event/Audit                            │
└────────────────┬────────────────────────┘
                 │
                SQLx
                 │
        ┌────────▼────────┐
        │                 │
 Development          Staging/Prod
        │                 │
      SQLite          PostgreSQL
```

Bu mimari projenin teknik source-of-truth karar setidir.
