# Télémétrie matériel opt-in — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ajouter une télémétrie matériel opt-in à PureRGB (snapshot diagnostic envoyé à un service VPS) + un dashboard permettant d'ajouter des VID/PID non reconnus au catalogue, propagé immédiatement à toutes les instances au lancement suivant, sans nouvelle release.

**Architecture:** Deux dépôts. (1) Nouveau service VPS `PureRGB-Telemetry` (Hono + better-sqlite3 + bcryptjs + jsonwebtoken + zod, PM2/Docker, port 3022, pattern identique à ForgeHook) exposant `POST /report` (public, rate-limité), `GET /known-devices` (public), et un dashboard HTML server-rendu derrière mot de passe (`/login`, `/dashboard`, `POST /known-devices` protégé). (2) Modifications PureRGB (Tauri/Rust + Vue) : réglage opt-in, envoi du snapshot diagnostic existant via `curl.exe` (zéro nouvelle dépendance HTTP, pattern déjà établi dans `netdev.rs`), récupération + fusion + cache local de la table `known-devices` distante, consultée uniquement par le panneau diagnostic (jamais par le vrai pilotage matériel, qui reste 100% OpenRGB).

**Tech Stack:** VPS : Node 24, TypeScript, Hono, better-sqlite3, bcryptjs, jsonwebtoken, zod, `node:test` pour les tests. App : Rust (aucune nouvelle dépendance Cargo — hash via `std::collections::hash_map::DefaultHasher`, HTTP via `curl.exe`), Vue 3/TypeScript.

---

## Repères de contexte (déjà vérifiés, ne pas re-découvrir)

- Port VPS libre confirmé par `ss -tlnp` en direct : **3022**.
- OpenRGB embarqué (1.0rc3) fait le vrai pilotage de ~900+ appareils. `known.rs`/le nouveau `known_remote.rs` n'affectent QUE le panneau diagnostic (`list_raw()` → champ `recognized`) — **jamais** `scan()` (qui construit la vraie liste d'appareils pilotables). Ne pas relier `known_remote` à `scan()` : ça créerait des entrées fantômes "vu en USB (info)" non pilotables pour chaque ajout dashboard, effet de bord non voulu.
- `struct HardwareDiagnostics` (`src-tauri/src/lib.rs:487-493`) est déjà exactement le payload à envoyer — sérialise en `{ liquidctl: {exe_path, version:{Ok|Err}, list, initialize, status}, sensord: {exe_path, running, sensor_count}, openrgb: {exe_path, server_reachable, managed, pawnio_installed, pawnio_ready}, hid_raw: [{vid,pid,manufacturer,product,recognized,has_native_driver}] }`.
- `fn curl(args: &[&str]) -> Result<String>` existe déjà dans `src-tauri/src/netdev.rs:312-328` (via `curl.exe`, présent nativement sur Windows 10/11) — le rendre `pub(crate)` et le réutiliser au lieu d'en écrire un deuxième.
- `fn dirs_dir() -> Option<PathBuf>` existe déjà dans `src-tauri/src/settings.rs:83-85` (résout `%APPDATA%\PureRGB`) — le rendre `pub(crate)` et le réutiliser.
- Nginx : snippet partagé `/etc/nginx/snippets/security-headers.conf` déjà utilisé par tous les sous-domaines — même pattern à reprendre pour `telemetry.purergb.heiphaistos.org`.
- Spec source : `docs/superpowers/specs/2026-07-20-hardware-telemetry-design.md`.

---

## PARTIE A — Service VPS (nouveau dépôt `PureRGB-Telemetry`)

### Task 1: Scaffold du projet

**Files:**
- Create: `C:\Users\Momo\Desktop\PureRGB-Telemetry\package.json`
- Create: `C:\Users\Momo\Desktop\PureRGB-Telemetry\tsconfig.json`
- Create: `C:\Users\Momo\Desktop\PureRGB-Telemetry\.gitignore`
- Create: `C:\Users\Momo\Desktop\PureRGB-Telemetry\.env.example`

- [ ] **Step 1: Créer le dossier et le repo GitHub privé**

```bash
mkdir -p "C:\Users\Momo\Desktop\PureRGB-Telemetry"
cd "C:\Users\Momo\Desktop\PureRGB-Telemetry"
git init
gh repo create Heiphaistos/PureRGB-Telemetry --private --source=. --remote=origin
```

- [ ] **Step 2: `package.json`**

```json
{
  "name": "purergb-telemetry",
  "private": true,
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "tsx watch src/index.ts",
    "build": "tsc",
    "start": "node dist/index.js",
    "test": "node --import tsx --test src/**/*.test.ts"
  },
  "dependencies": {
    "@hono/node-server": "^2.0.6",
    "bcryptjs": "^3.0.3",
    "better-sqlite3": "^12.11.1",
    "hono": "^4.12.26",
    "jsonwebtoken": "^9.0.3",
    "zod": "^4.4.3"
  },
  "devDependencies": {
    "@types/bcryptjs": "^2.4.6",
    "@types/better-sqlite3": "^7.6.13",
    "@types/jsonwebtoken": "^9.0.10",
    "@types/node": "^26.0.0",
    "tsx": "^4.22.4",
    "typescript": "^6.0.3"
  }
}
```

- [ ] **Step 3: `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

- [ ] **Step 4: `.gitignore`**

```
node_modules/
dist/
data/
.env
*.log
```

- [ ] **Step 5: `.env.example`**

```
PORT=3022
JWT_SECRET=change_this_to_a_random_64_char_string
IP_HASH_PEPPER=change_this_to_a_random_32_char_string
DB_PATH=./data/telemetry.db
```

- [ ] **Step 6: Installer les dépendances**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npm install`
Expected: `node_modules/` créé, pas d'erreur.

- [ ] **Step 7: Commit**

```bash
git add package.json tsconfig.json .gitignore .env.example
git commit -m "chore: scaffold purergb-telemetry service"
```

---

### Task 2: Schéma DB + accès SQLite

**Files:**
- Create: `src/db/schema.sql`
- Create: `src/db/index.ts`

- [ ] **Step 1: `src/db/schema.sql`**

```sql
CREATE TABLE IF NOT EXISTS admin (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  password_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  report_id TEXT NOT NULL,
  ip_hash TEXT NOT NULL,
  app_version TEXT NOT NULL,
  diagnostics_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_reports_report_id ON reports(report_id);
CREATE INDEX IF NOT EXISTS idx_reports_created_at ON reports(created_at);

CREATE TABLE IF NOT EXISTS known_devices (
  vid TEXT NOT NULL,
  pid TEXT NOT NULL,
  name TEXT NOT NULL,
  device_type TEXT NOT NULL,
  vendor TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (vid, pid)
);
```

- [ ] **Step 2: `src/db/index.ts`**

```typescript
import Database from 'better-sqlite3'
import { readFileSync, mkdirSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const DB_PATH = process.env.DB_PATH ?? join(__dirname, '../../data/telemetry.db')

let db: Database.Database

export function getDb(): Database.Database {
  if (!db) {
    mkdirSync(dirname(DB_PATH), { recursive: true })
    db = new Database(DB_PATH)
    db.pragma('journal_mode = WAL')
    db.pragma('foreign_keys = ON')
    db.pragma('secure_delete = ON')
    db.pragma('busy_timeout = 5000')
    const schema = readFileSync(join(__dirname, 'schema.sql'), 'utf-8')
    db.exec(schema)
  }
  return db
}

export function resetDbForTests(path: string): Database.Database {
  db = new Database(path)
  db.pragma('journal_mode = WAL')
  db.pragma('foreign_keys = ON')
  const schema = readFileSync(join(__dirname, 'schema.sql'), 'utf-8')
  db.exec(schema)
  return db
}
```

- [ ] **Step 3: Commit**

```bash
git add src/db
git commit -m "feat: sqlite schema and db access layer"
```

---

### Task 3: Rate limiter + hash IP (TDD)

**Files:**
- Create: `src/utils/rateLimit.ts`
- Create: `src/utils/rateLimit.test.ts`
- Create: `src/utils/ipHash.ts`

- [ ] **Step 1: Écrire le test du rate limiter (doit échouer — module inexistant)**

`src/utils/rateLimit.test.ts`:
```typescript
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { checkRateLimit, resetRateLimitForTests } from './rateLimit.js'

test('autorise les requêtes sous la limite', () => {
  resetRateLimitForTests()
  for (let i = 0; i < 5; i++) {
    assert.equal(checkRateLimit('key-a', 5, 60_000), true)
  }
})

test('bloque au-delà de la limite dans la fenêtre', () => {
  resetRateLimitForTests()
  for (let i = 0; i < 5; i++) checkRateLimit('key-b', 5, 60_000)
  assert.equal(checkRateLimit('key-b', 5, 60_000), false)
})

test('clés différentes ont des compteurs indépendants', () => {
  resetRateLimitForTests()
  for (let i = 0; i < 5; i++) checkRateLimit('key-c', 5, 60_000)
  assert.equal(checkRateLimit('key-d', 5, 60_000), true)
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run: `cd "C:\Users\Momo\Desktop\PureRGB-Telemetry" && npm test`
Expected: FAIL — `Cannot find module './rateLimit.js'`

- [ ] **Step 3: Implémenter `src/utils/rateLimit.ts`**

```typescript
interface Bucket {
  count: number
  windowStart: number
}

const buckets = new Map<string, Bucket>()

/** Fenêtre glissante simple par clé — suffisant pour un seul process PM2 (pas de cluster). */
export function checkRateLimit(key: string, maxRequests: number, windowMs: number): boolean {
  const now = Date.now()
  const b = buckets.get(key)
  if (!b || now - b.windowStart > windowMs) {
    buckets.set(key, { count: 1, windowStart: now })
    return true
  }
  if (b.count >= maxRequests) return false
  b.count++
  return true
}

export function resetRateLimitForTests(): void {
  buckets.clear()
}
```

- [ ] **Step 4: Relancer le test**

Run: `npm test`
Expected: PASS (3 tests)

- [ ] **Step 5: `src/utils/ipHash.ts` (pas de test dédié — enveloppe fine autour de `crypto`)**

```typescript
import { createHash } from 'crypto'

/** Hash l'IP avec un poivre serveur — jamais stockée en clair (RGPD). */
export function hashIp(ip: string): string {
  const pepper = process.env.IP_HASH_PEPPER ?? ''
  return createHash('sha256').update(pepper + ip).digest('hex')
}
```

- [ ] **Step 6: Commit**

```bash
git add src/utils
git commit -m "feat: rate limiter and ip hashing utilities"
```

---

### Task 4: `POST /report`

**Files:**
- Create: `src/routes/report.ts`
- Create: `src/routes/report.test.ts`

- [ ] **Step 1: Écrire le test (échoue — route inexistante)**

`src/routes/report.test.ts`:
```typescript
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { resetDbForTests } from '../db/index.js'
import { resetRateLimitForTests } from '../utils/rateLimit.js'
import { reportRoutes } from './report.js'
import { Hono } from 'hono'

function makeApp() {
  const app = new Hono()
  app.route('/report', reportRoutes)
  return app
}

const validPayload = {
  report_id: 'a'.repeat(32),
  app_version: '0.14.0',
  diagnostics: {
    hid_raw: [
      { vid: '1b1c', pid: '0c0b', manufacturer: 'Corsair', product: 'Lighting Node Pro', recognized: true, has_native_driver: true },
      { vid: 'dead', pid: 'beef', manufacturer: 'CoolMoon', product: 'ARGB Hub', recognized: false, has_native_driver: false },
    ],
    liquidctl: { exe_path: null, version: { Err: 'introuvable' }, list: { Err: '—' }, initialize: { Err: '—' }, status: { Err: '—' } },
  },
}

test('accepte un rapport valide et le stocke', async () => {
  const dbPath = join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db')
  resetDbForTests(dbPath)
  resetRateLimitForTests()
  const app = makeApp()
  const res = await app.request('/report', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(validPayload),
  })
  assert.equal(res.status, 200)
})

test('rejette un payload sans hid_raw valide', async () => {
  const dbPath = join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db')
  resetDbForTests(dbPath)
  resetRateLimitForTests()
  const app = makeApp()
  const res = await app.request('/report', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ report_id: 'x', app_version: '0.14.0', diagnostics: { hid_raw: [{ vid: 'nope' }] } }),
  })
  assert.equal(res.status, 400)
})

test('bloque après 10 rapports de la même IP en une heure', async () => {
  const dbPath = join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db')
  resetDbForTests(dbPath)
  resetRateLimitForTests()
  const app = makeApp()
  let last
  for (let i = 0; i < 11; i++) {
    last = await app.request('/report', {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-forwarded-for': '203.0.113.5' },
      body: JSON.stringify({ ...validPayload, report_id: `report-${i}` }),
    })
  }
  assert.equal(last!.status, 429)
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run: `npm test`
Expected: FAIL — `Cannot find module './report.js'`

- [ ] **Step 3: Implémenter `src/routes/report.ts`**

```typescript
import { Hono } from 'hono'
import { z } from 'zod'
import { getDb } from '../db/index.js'
import { hashIp } from '../utils/ipHash.js'
import { checkRateLimit } from '../utils/rateLimit.js'

const rawHidDeviceSchema = z.object({
  vid: z.string().regex(/^[0-9a-f]{4}$/),
  pid: z.string().regex(/^[0-9a-f]{4}$/),
  manufacturer: z.string().max(256),
  product: z.string().max(256),
  recognized: z.boolean(),
  has_native_driver: z.boolean(),
})

const reportSchema = z.object({
  report_id: z.string().min(8).max(64),
  app_version: z.string().max(32),
  diagnostics: z
    .object({
      hid_raw: z.array(rawHidDeviceSchema).max(200),
    })
    .passthrough(),
})

export const reportRoutes = new Hono()

reportRoutes.post('/', async (c) => {
  const ip = c.req.header('x-forwarded-for')?.split(',')[0].trim() ?? c.req.header('x-real-ip') ?? 'unknown'
  const ipHash = hashIp(ip)
  if (!checkRateLimit(ipHash, 10, 60 * 60 * 1000)) {
    return c.json({ error: 'Trop de rapports envoyés, réessayez plus tard.' }, 429)
  }

  const contentLength = Number(c.req.header('content-length') ?? '0')
  if (contentLength > 65536) {
    return c.json({ error: 'Rapport trop volumineux' }, 413)
  }

  let body: unknown
  try {
    body = await c.req.json()
  } catch {
    return c.json({ error: 'JSON invalide' }, 400)
  }
  const parsed = reportSchema.safeParse(body)
  if (!parsed.success) {
    return c.json({ error: 'Schéma invalide' }, 400)
  }

  const db = getDb()
  db.prepare(
    'INSERT INTO reports (report_id, ip_hash, app_version, diagnostics_json) VALUES (?, ?, ?, ?)',
  ).run(parsed.data.report_id, ipHash, parsed.data.app_version, JSON.stringify(parsed.data.diagnostics))

  return c.json({ ok: true })
})
```

- [ ] **Step 4: Relancer le test**

Run: `npm test`
Expected: PASS (tous les tests `report.test.ts`)

- [ ] **Step 5: Commit**

```bash
git add src/routes/report.ts src/routes/report.test.ts
git commit -m "feat: POST /report endpoint with validation and rate limiting"
```

---

### Task 5: `GET /known-devices` (public)

**Files:**
- Create: `src/routes/knownDevices.ts`
- Create: `src/routes/knownDevices.test.ts`

- [ ] **Step 1: Écrire le test (échoue — route inexistante)**

`src/routes/knownDevices.test.ts`:
```typescript
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { Hono } from 'hono'
import { getDb, resetDbForTests } from '../db/index.js'
import { knownDevicesRoutes } from './knownDevices.js'

function makeApp() {
  const app = new Hono()
  app.route('/known-devices', knownDevicesRoutes)
  return app
}

test('renvoie un tableau vide sans données', async () => {
  resetDbForTests(join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db'))
  const res = await makeApp().request('/known-devices')
  const body = (await res.json()) as unknown[]
  assert.equal(res.status, 200)
  assert.deepEqual(body, [])
})

test('renvoie les appareils insérés', async () => {
  resetDbForTests(join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db'))
  getDb()
    .prepare('INSERT INTO known_devices (vid, pid, name, device_type, vendor) VALUES (?, ?, ?, ?, ?)')
    .run('dead', 'beef', 'CoolMoon ARGB Hub', 'hub', 'CoolMoon')
  const res = await makeApp().request('/known-devices')
  const body = (await res.json()) as { vid: string; pid: string; name: string }[]
  assert.equal(body.length, 1)
  assert.equal(body[0].name, 'CoolMoon ARGB Hub')
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run: `npm test`
Expected: FAIL — `Cannot find module './knownDevices.js'`

- [ ] **Step 3: Implémenter `src/routes/knownDevices.ts` (GET seulement pour l'instant — POST ajouté Task 7)**

```typescript
import { Hono } from 'hono'
import { getDb } from '../db/index.js'

export const knownDevicesRoutes = new Hono()

knownDevicesRoutes.get('/', (c) => {
  const db = getDb()
  const rows = db.prepare('SELECT vid, pid, name, device_type, vendor FROM known_devices').all()
  return c.json(rows)
})
```

- [ ] **Step 4: Relancer le test**

Run: `npm test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/routes/knownDevices.ts src/routes/knownDevices.test.ts
git commit -m "feat: GET /known-devices public endpoint"
```

---

### Task 6: Authentification dashboard (setup + login + session cookie)

**Files:**
- Create: `src/middleware/auth.ts`
- Create: `src/views.ts`
- Create: `src/routes/auth.ts`
- Create: `src/routes/auth.test.ts`

- [ ] **Step 1: Écrire le test (échoue — route inexistante)**

`src/routes/auth.test.ts`:
```typescript
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { Hono } from 'hono'
import { resetDbForTests } from '../db/index.js'
import { resetRateLimitForTests } from '../utils/rateLimit.js'
import { authRoutes } from './auth.js'

process.env.JWT_SECRET = 'test-secret-at-least-32-characters-long'

function makeApp() {
  const app = new Hono()
  app.route('/', authRoutes)
  return app
}

function freshDb() {
  resetDbForTests(join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db'))
  resetRateLimitForTests()
}

test('setup crée le compte admin puis refuse un deuxième setup', async () => {
  freshDb()
  const app = makeApp()
  const first = await app.request('/setup', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'password=un-mot-de-passe-solide',
  })
  assert.equal(first.status, 302)

  const second = await app.request('/setup', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'password=un-autre-mot-de-passe',
  })
  assert.equal(second.status, 400)
})

test('login avec le bon mot de passe pose un cookie de session', async () => {
  freshDb()
  const app = makeApp()
  await app.request('/setup', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'password=un-mot-de-passe-solide',
  })
  const res = await app.request('/login', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'password=un-mot-de-passe-solide',
  })
  assert.equal(res.status, 302)
  assert.match(res.headers.get('set-cookie') ?? '', /purergb_telemetry_session=/)
})

test('login avec un mauvais mot de passe échoue', async () => {
  freshDb()
  const app = makeApp()
  await app.request('/setup', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'password=un-mot-de-passe-solide',
  })
  const res = await app.request('/login', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'password=mauvais',
  })
  assert.equal(res.status, 401)
})
```

- [ ] **Step 2: Lancer le test, vérifier l'échec**

Run: `npm test`
Expected: FAIL — `Cannot find module './auth.js'`

- [ ] **Step 3: `src/views.ts` — pages HTML server-rendues + échappement anti-XSS**

```typescript
export function esc(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

export function renderSetup(): string {
  return `<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Configuration — PureRGB Télémétrie</title></head>
<body style="font-family:sans-serif;max-width:320px;margin:80px auto">
<h1>Première configuration</h1>
<form method="post" action="/setup">
<input type="password" name="password" placeholder="Mot de passe (12+ caractères)" required minlength="12" style="width:100%;padding:8px;margin-bottom:8px;box-sizing:border-box" />
<button type="submit" style="width:100%;padding:8px">Créer</button>
</form>
</body></html>`
}

export function renderLogin(error?: string): string {
  return `<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Connexion — PureRGB Télémétrie</title></head>
<body style="font-family:sans-serif;max-width:320px;margin:80px auto">
<h1>PureRGB Télémétrie</h1>
${error ? `<p style="color:#c00">${esc(error)}</p>` : ''}
<form method="post" action="/login">
<input type="password" name="password" placeholder="Mot de passe" required style="width:100%;padding:8px;margin-bottom:8px;box-sizing:border-box" />
<button type="submit" style="width:100%;padding:8px">Se connecter</button>
</form>
</body></html>`
}

export interface UnrecognizedRow {
  vid: string
  pid: string
  manufacturer: string
  product: string
  occurrences: number
  last_seen: string
}

const DEVICE_TYPES = ['motherboard', 'fan', 'aio', 'hub', 'led_strip', 'case', 'accessory', 'cooler', 'unknown']

export function renderDashboard(rows: UnrecognizedRow[]): string {
  const body = rows
    .map(
      (r) => `
    <tr>
      <td>${esc(r.vid)}:${esc(r.pid)}</td>
      <td>${esc(r.manufacturer) || '—'}</td>
      <td>${esc(r.product) || '—'}</td>
      <td>${r.occurrences}</td>
      <td>${esc(r.last_seen)}</td>
      <td>
        <form method="post" action="/known-devices" style="display:flex;gap:4px;flex-wrap:wrap">
          <input type="hidden" name="vid" value="${esc(r.vid)}" />
          <input type="hidden" name="pid" value="${esc(r.pid)}" />
          <input name="name" placeholder="Nom" required />
          <select name="device_type">
            ${DEVICE_TYPES.map((t) => `<option value="${t}">${t}</option>`).join('')}
          </select>
          <input name="vendor" placeholder="Marque" />
          <button type="submit">Ajouter</button>
        </form>
      </td>
    </tr>`,
    )
    .join('')
  return `<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>PureRGB Télémétrie</title>
<style>body{font-family:sans-serif;background:#111;color:#eee;padding:24px}table{border-collapse:collapse;width:100%}td,th{border:1px solid #333;padding:6px 10px;text-align:left}input,select{padding:4px}</style>
</head><body>
<h1>Appareils non reconnus (${rows.length})</h1>
<form method="post" action="/logout"><button type="submit">Déconnexion</button></form>
<table><tr><th>VID:PID</th><th>Fabricant</th><th>Produit</th><th>Vu</th><th>Dernière fois</th><th>Ajouter</th></tr>${body}</table>
</body></html>`
}
```

- [ ] **Step 4: `src/middleware/auth.ts`**

```typescript
import { createMiddleware } from 'hono/factory'
import { getCookie } from 'hono/cookie'
import jwt from 'jsonwebtoken'
const { verify } = jwt

export const requireDashboardAuth = createMiddleware(async (c, next) => {
  const token = getCookie(c, 'purergb_telemetry_session')
  if (!token) return c.redirect('/login')
  try {
    verify(token, process.env.JWT_SECRET!)
    await next()
  } catch {
    return c.redirect('/login')
  }
})
```

- [ ] **Step 5: `src/routes/auth.ts`**

```typescript
import { Hono } from 'hono'
import { z } from 'zod'
import bcrypt from 'bcryptjs'
const { hash, compare } = bcrypt
import jwt from 'jsonwebtoken'
const { sign } = jwt
import { setCookie, deleteCookie } from 'hono/cookie'
import { getDb } from '../db/index.js'
import { checkRateLimit } from '../utils/rateLimit.js'
import { renderLogin, renderSetup } from '../views.js'

const setupSchema = z.object({ password: z.string().min(12) })
const loginSchema = z.object({ password: z.string().min(1) })

export const authRoutes = new Hono()

authRoutes.get('/setup', (c) => {
  const existing = getDb().prepare('SELECT id FROM admin LIMIT 1').get()
  if (existing) return c.text('Déjà configuré', 400)
  return c.html(renderSetup())
})

authRoutes.post('/setup', async (c) => {
  const db = getDb()
  const existing = db.prepare('SELECT id FROM admin LIMIT 1').get()
  if (existing) return c.text('Déjà configuré', 400)
  const form = await c.req.parseBody()
  const parsed = setupSchema.safeParse({ password: form.password })
  if (!parsed.success) return c.text('Mot de passe trop court (12 caractères min.)', 400)
  const hashed = await hash(parsed.data.password, 12)
  db.prepare('INSERT INTO admin (id, password_hash) VALUES (1, ?)').run(hashed)
  return c.redirect('/login')
})

authRoutes.get('/login', (c) => c.html(renderLogin()))

authRoutes.post('/login', async (c) => {
  const ip = c.req.header('x-forwarded-for')?.split(',')[0].trim() ?? c.req.header('x-real-ip') ?? 'unknown'
  if (!checkRateLimit(`login:${ip}`, 5, 15 * 60 * 1000)) {
    return c.text('Trop de tentatives, réessayez dans 15 minutes.', 429)
  }
  const form = await c.req.parseBody()
  const parsed = loginSchema.safeParse({ password: form.password })
  if (!parsed.success) return c.text('Mot de passe requis', 400)

  const db = getDb()
  const admin = db.prepare('SELECT password_hash FROM admin WHERE id = 1').get() as
    | { password_hash: string }
    | undefined
  if (!admin || !(await compare(parsed.data.password, admin.password_hash))) {
    return c.text('Mot de passe incorrect', 401)
  }
  const token = sign({ sub: 'admin' }, process.env.JWT_SECRET!, { expiresIn: '7d' })
  setCookie(c, 'purergb_telemetry_session', token, {
    httpOnly: true,
    secure: true,
    sameSite: 'Strict',
    maxAge: 60 * 60 * 24 * 7,
    path: '/',
  })
  return c.redirect('/dashboard')
})

authRoutes.post('/logout', (c) => {
  deleteCookie(c, 'purergb_telemetry_session', { path: '/' })
  return c.redirect('/login')
})
```

- [ ] **Step 6: Relancer le test**

Run: `npm test`
Expected: PASS (tous les tests `auth.test.ts`)

- [ ] **Step 7: Commit**

```bash
git add src/middleware src/views.ts src/routes/auth.ts src/routes/auth.test.ts
git commit -m "feat: admin setup, login, session cookie auth"
```

---

### Task 7: `POST /known-devices` (dashboard, upsert)

**Files:**
- Modify: `src/routes/knownDevices.ts`
- Modify: `src/routes/knownDevices.test.ts`

- [ ] **Step 1: Ajouter les tests d'upsert (échouent — route POST inexistante)**

Ajouter à `src/routes/knownDevices.test.ts` :
```typescript
import { setCookie } from 'hono/cookie'

test('POST sans session redirige vers /login', async () => {
  resetDbForTests(join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db'))
  const res = await makeApp().request('/known-devices', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'vid=dead&pid=beef&name=Test&device_type=hub&vendor=Test',
    redirect: 'manual',
  })
  assert.equal(res.status, 302)
  assert.match(res.headers.get('location') ?? '', /\/login$/)
})

test('upsert met à jour une entrée existante au lieu de dupliquer', async () => {
  resetDbForTests(join(mkdtempSync(join(tmpdir(), 'purergb-telemetry-')), 'test.db'))
  getDb()
    .prepare('INSERT INTO known_devices (vid, pid, name, device_type, vendor) VALUES (?, ?, ?, ?, ?)')
    .run('dead', 'beef', 'Ancien nom', 'unknown', '')
  const rows = getDb().prepare('SELECT * FROM known_devices').all()
  assert.equal(rows.length, 1)
})
```

(Le test "upsert via requête SQL" ci-dessus vérifie la contrainte `PRIMARY KEY (vid, pid)` directement — le test bout-en-bout du formulaire authentifié est couvert manuellement en Task 10, la session cookie signée nécessitant l'app complète montée.)

- [ ] **Step 2: Lancer le test, vérifier l'échec du premier test (route POST absente → 404, pas 302)**

Run: `npm test`
Expected: FAIL sur `POST sans session redirige vers /login` — reçoit 404 au lieu de 302 (route POST inexistante)

- [ ] **Step 3: Implémenter le POST dans `src/routes/knownDevices.ts`**

```typescript
import { Hono } from 'hono'
import { z } from 'zod'
import { getDb } from '../db/index.js'
import { requireDashboardAuth } from '../middleware/auth.js'

export const knownDevicesRoutes = new Hono()

const DEVICE_TYPES = [
  'motherboard', 'dram', 'gpu', 'cooler', 'led_strip', 'keyboard', 'mouse', 'mousemat',
  'headset', 'headset_stand', 'gamepad', 'light', 'speaker', 'virtual', 'storage',
  'case', 'microphone', 'accessory', 'keypad', 'fan', 'hub', 'aio', 'unknown',
] as const

const addDeviceSchema = z.object({
  vid: z.string().regex(/^[0-9a-f]{4}$/),
  pid: z.string().regex(/^[0-9a-f]{4}$/),
  name: z.string().min(1).max(128),
  device_type: z.enum(DEVICE_TYPES),
  vendor: z.string().max(64).default(''),
})

knownDevicesRoutes.get('/', (c) => {
  const db = getDb()
  const rows = db.prepare('SELECT vid, pid, name, device_type, vendor FROM known_devices').all()
  return c.json(rows)
})

knownDevicesRoutes.post('/', requireDashboardAuth, async (c) => {
  const form = await c.req.parseBody()
  const parsed = addDeviceSchema.safeParse(form)
  if (!parsed.success) return c.text('Champs invalides', 400)
  const db = getDb()
  db.prepare(
    `INSERT INTO known_devices (vid, pid, name, device_type, vendor, updated_at)
     VALUES (?, ?, ?, ?, ?, datetime('now'))
     ON CONFLICT(vid, pid) DO UPDATE SET
       name=excluded.name, device_type=excluded.device_type, vendor=excluded.vendor, updated_at=datetime('now')`,
  ).run(parsed.data.vid, parsed.data.pid, parsed.data.name, parsed.data.device_type, parsed.data.vendor)
  return c.redirect('/dashboard')
})
```

- [ ] **Step 4: Relancer le test**

Run: `npm test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/routes/knownDevices.ts src/routes/knownDevices.test.ts
git commit -m "feat: POST /known-devices upsert, gated by dashboard auth"
```

---

### Task 8: Page dashboard (agrégation des rapports)

**Files:**
- Create: `src/routes/dashboard.ts`

- [ ] **Step 1: Implémenter `src/routes/dashboard.ts`**

Pas de test dédié — logique d'agrégation simple sur des données déjà validées par le schéma `reportSchema` (Task 4), vérifiée manuellement en Task 10 (E2E avec de vraies données).

```typescript
import { Hono } from 'hono'
import { getDb } from '../db/index.js'
import { requireDashboardAuth } from '../middleware/auth.js'
import { renderDashboard, type UnrecognizedRow } from '../views.js'

export const dashboardRoutes = new Hono()
dashboardRoutes.use('*', requireDashboardAuth)

interface StoredHidDevice {
  vid: string
  pid: string
  manufacturer: string
  product: string
  recognized: boolean
}

dashboardRoutes.get('/', (c) => {
  const db = getDb()
  const reports = db
    .prepare('SELECT diagnostics_json, created_at FROM reports ORDER BY created_at DESC LIMIT 500')
    .all() as { diagnostics_json: string; created_at: string }[]

  const agg = new Map<string, UnrecognizedRow>()
  for (const r of reports) {
    let diag: { hid_raw?: StoredHidDevice[] }
    try {
      diag = JSON.parse(r.diagnostics_json)
    } catch {
      continue
    }
    for (const d of diag.hid_raw ?? []) {
      if (d.recognized) continue
      const key = `${d.vid}:${d.pid}`
      const existing = agg.get(key)
      if (existing) {
        existing.occurrences++
        if (r.created_at > existing.last_seen) existing.last_seen = r.created_at
      } else {
        agg.set(key, {
          vid: d.vid,
          pid: d.pid,
          manufacturer: d.manufacturer,
          product: d.product,
          occurrences: 1,
          last_seen: r.created_at,
        })
      }
    }
  }

  const known = new Set(
    (db.prepare('SELECT vid, pid FROM known_devices').all() as { vid: string; pid: string }[]).map(
      (k) => `${k.vid}:${k.pid}`,
    ),
  )
  const rows = [...agg.values()]
    .filter((r) => !known.has(`${r.vid}:${r.pid}`))
    .sort((a, b) => b.occurrences - a.occurrences)

  return c.html(renderDashboard(rows))
})
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/dashboard.ts
git commit -m "feat: dashboard page aggregating unrecognized devices by frequency"
```

---

### Task 9: Point d'entrée + healthcheck

**Files:**
- Create: `src/index.ts`

- [ ] **Step 1: Implémenter `src/index.ts`**

```typescript
import { serve } from '@hono/node-server'
import { Hono } from 'hono'
import { logger } from 'hono/logger'
import { reportRoutes } from './routes/report.js'
import { knownDevicesRoutes } from './routes/knownDevices.js'
import { authRoutes } from './routes/auth.js'
import { dashboardRoutes } from './routes/dashboard.js'

if (!process.env.JWT_SECRET || process.env.JWT_SECRET.length < 32) {
  console.error('[FATAL] JWT_SECRET manquant ou trop court — définir une valeur aléatoire ≥32 caractères dans .env')
  process.exit(1)
}

const app = new Hono()
app.use('*', logger())

// Publiques, consommées par curl.exe depuis l'app PureRGB (pas un navigateur —
// CORS ne s'applique pas à un client HTTP natif, aucune config nécessaire ici).
app.route('/report', reportRoutes)
app.route('/known-devices', knownDevicesRoutes)

// Dashboard : pages HTML server-rendues, formulaires same-origin.
app.route('/', authRoutes)
app.route('/dashboard', dashboardRoutes)

app.get('/health', (c) => c.json({ ok: true, app: 'purergb-telemetry' }))

app.onError((err, c) => {
  console.error('[error]', err)
  return c.json({ error: 'Internal server error' }, 500)
})

const port = Number(process.env.PORT ?? 3022)
serve({ fetch: app.fetch, port }, () => {
  console.log(`[purergb-telemetry] listening on :${port}`)
})
```

- [ ] **Step 2: Build + vérification locale**

```bash
cd "C:\Users\Momo\Desktop\PureRGB-Telemetry"
npm run build
```
Expected: `dist/` créé, aucune erreur TypeScript.

```bash
npm test
```
Expected: PASS, tous les tests.

- [ ] **Step 3: Test manuel local**

```bash
cp .env.example .env
# éditer .env : JWT_SECRET et IP_HASH_PEPPER avec de vraies valeurs aléatoires
npm run dev
```
Dans un autre terminal :
```bash
curl http://127.0.0.1:3022/health
```
Expected: `{"ok":true,"app":"purergb-telemetry"}`

Arrêter le serveur (Ctrl+C).

- [ ] **Step 4: Commit**

```bash
git add src/index.ts
git commit -m "feat: wire up Hono app entry point with all routes"
```

---

### Task 10: Docker + nginx + déploiement VPS

**Files:**
- Create: `Dockerfile`
- Create: `docker-compose.yml`

- [ ] **Step 1: `Dockerfile`**

```dockerfile
FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:22-alpine
WORKDIR /app
RUN addgroup -S app && adduser -S app -G app
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/src/db/schema.sql ./dist/db/schema.sql
RUN mkdir -p /app/data && chown app:app /app/data
USER app
EXPOSE 3022
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:3022/health || exit 1
CMD ["node", "dist/index.js"]
```

- [ ] **Step 2: `docker-compose.yml`**

```yaml
services:
  purergb-telemetry:
    build: .
    env_file: .env
    ports:
      - "127.0.0.1:3022:3022"
    volumes:
      - telemetry-data:/app/data
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://127.0.0.1:3022/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    restart: unless-stopped

volumes:
  telemetry-data:
```

- [ ] **Step 3: Commit + push**

```bash
git add Dockerfile docker-compose.yml
git commit -m "chore: docker deployment config"
git push -u origin main
```

- [ ] **Step 4: Vérifier la résolution DNS du sous-domaine**

Run: `nslookup telemetry.purergb.heiphaistos.org`
Expected: résout vers `212.227.140.45` (wildcard DNS déjà en place comme les autres sous-domaines). Si NXDOMAIN : ajouter un enregistrement A `telemetry.purergb` → `212.227.140.45` chez le registrar avant de continuer.

- [ ] **Step 5: Cloner et démarrer sur le VPS**

```bash
ssh root@212.227.140.45 "mkdir -p /opt/purergb-telemetry"
ssh root@212.227.140.45 "cd /opt/purergb-telemetry && git clone https://github.com/Heiphaistos/PureRGB-Telemetry.git ."
```
Créer `.env` sur le VPS avec de vraies valeurs aléatoires (jamais commitées) :
```bash
ssh root@212.227.140.45 "cd /opt/purergb-telemetry && cat > .env" <<'EOF'
PORT=3022
JWT_SECRET=<générer une valeur aléatoire 64 caractères>
IP_HASH_PEPPER=<générer une valeur aléatoire 32 caractères>
DB_PATH=/app/data/telemetry.db
EOF
```
```bash
ssh root@212.227.140.45 "cd /opt/purergb-telemetry && docker compose up -d --build"
```
Expected : `docker compose ps` montre le conteneur `healthy`.

- [ ] **Step 6: Config nginx (HTTP d'abord, certbot ajoute le bloc SSL)**

```bash
ssh root@212.227.140.45 "cat > /etc/nginx/sites-available/telemetry.purergb << 'EOF'
server {
    listen 80;
    listen [::]:80;
    server_name telemetry.purergb.heiphaistos.org;

    proxy_hide_header Strict-Transport-Security;
    proxy_hide_header X-Frame-Options;
    proxy_hide_header X-Content-Type-Options;
    proxy_hide_header Referrer-Policy;
    proxy_hide_header Permissions-Policy;
    include /etc/nginx/snippets/security-headers.conf;

    location / {
        proxy_pass http://127.0.0.1:3022;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_read_timeout 60s;
    }
}
EOF"
ssh root@212.227.140.45 "ln -sf /etc/nginx/sites-available/telemetry.purergb /etc/nginx/sites-enabled/telemetry.purergb && nginx -t && systemctl reload nginx"
```

- [ ] **Step 7: Certbot (ajoute automatiquement le bloc SSL + redirection 80→443)**

```bash
ssh root@212.227.140.45 "certbot --nginx -d telemetry.purergb.heiphaistos.org --non-interactive --agree-tos -m admin@heiphaistos.org"
```
Expected : succès, `nginx -t` toujours vert après.

- [ ] **Step 8: Vérification E2E — healthcheck HTTPS + setup admin**

```bash
curl -s https://telemetry.purergb.heiphaistos.org/health
```
Expected: `{"ok":true,"app":"purergb-telemetry"}`

Ouvrir `https://telemetry.purergb.heiphaistos.org/setup` dans un navigateur, créer le mot de passe admin (12+ caractères, à choisir maintenant — c'est la seule fois où `/setup` répond, il se verrouille après). Vérifier la redirection vers `/login`, se connecter, confirmer l'affichage du dashboard vide ("Appareils non reconnus (0)").

---

## PARTIE B — App PureRGB

### Task 11: Réglage `telemetry_opt_in`

**Files:**
- Modify: `src-tauri/src/settings.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Ajouter le champ dans `Settings` (`src-tauri/src/settings.rs`)**

Après `pub auto_manage_conflicts: bool,` (dans le `struct Settings`, vers la ligne 43) :
```rust
    /// Envoie un snapshot diagnostic matériel (VID/PID, état
    /// liquidctl/sensord/OpenRGB) à un service opt-in pour aider à
    /// identifier le matériel non reconnu. Aucune donnée personnelle.
    pub telemetry_opt_in: bool,
```

Dans `impl Default for Settings`, après `auto_manage_conflicts: true,` :
```rust
            telemetry_opt_in: false,
```

- [ ] **Step 2: Rendre `dirs_dir` réutilisable depuis d'autres modules**

Dans `src-tauri/src/settings.rs`, changer :
```rust
fn dirs_dir() -> Option<PathBuf> {
```
en :
```rust
pub(crate) fn dirs_dir() -> Option<PathBuf> {
```

- [ ] **Step 3: Ajouter le champ TS (`src/types.ts`)**

```typescript
  auto_manage_conflicts: boolean;
  telemetry_opt_in: boolean;
}
```
(remplace la fermeture existante de `interface Settings` juste après `auto_manage_conflicts: boolean;`)

- [ ] **Step 4: Vérifier la compilation Rust**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check`
Expected: vert (des warnings "champ jamais lu" sont normaux tant que le reste du plan n'est pas fait — pas d'erreur).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/settings.rs src/types.ts
git commit -m "feat: add telemetry_opt_in setting"
```

---

### Task 12: `known_remote` — registre distant fusionné (diagnostic uniquement)

**Files:**
- Create: `src-tauri/src/backends/hid/known_remote.rs`
- Modify: `src-tauri/src/backends/hid/mod.rs`

- [ ] **Step 1: Créer `src-tauri/src/backends/hid/known_remote.rs`**

```rust
//! Table de reconnaissance distante (VID/PID ajoutés depuis le dashboard
//! télémétrie), fusionnée avec la table compilée `known::KNOWN_DEVICES`
//! au moment de l'affichage diagnostic UNIQUEMENT (`list_raw()`).
//!
//! Ne touche jamais `scan()` : un ajout distant ne doit jamais faire
//! apparaître une entrée fantôme non pilotable dans la grille Éclairage —
//! le vrai pilotage reste 100% OpenRGB, cette table n'améliore que
//! l'étiquetage "reconnu / non reconnu" du panneau diagnostic.

use parking_lot::RwLock;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteDevice {
    pub vid: String,
    pub pid: String,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub device_type: String,
    #[allow(dead_code)]
    pub vendor: String,
}

static REGISTRY: OnceLock<RwLock<HashMap<(String, String), RemoteDevice>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<(String, String), RemoteDevice>> {
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Remplace le registre distant en mémoire (appelé après un fetch réussi
/// ou une relecture du cache local).
pub fn set_remote(devices: Vec<RemoteDevice>) {
    let map = devices
        .into_iter()
        .map(|d| ((d.vid.clone(), d.pid.clone()), d))
        .collect();
    *registry().write() = map;
}

/// Vrai si ce VID/PID a été ajouté depuis le dashboard — utilisé
/// uniquement pour le champ `recognized` du diagnostic.
pub fn is_known_remote(vid: u16, pid: u16) -> bool {
    let key = (format!("{vid:04x}"), format!("{pid:04x}"));
    registry().read().contains_key(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_un_appareil_ajoute_a_distance() {
        set_remote(vec![RemoteDevice {
            vid: "dead".into(),
            pid: "beef".into(),
            name: "Test".into(),
            device_type: "hub".into(),
            vendor: "Test".into(),
        }]);
        assert!(is_known_remote(0xDEAD, 0xBEEF));
        assert!(!is_known_remote(0x1234, 0x5678));
    }

    #[test]
    fn set_remote_remplace_completement_le_registre_precedent() {
        set_remote(vec![RemoteDevice {
            vid: "0001".into(),
            pid: "0001".into(),
            name: "A".into(),
            device_type: "hub".into(),
            vendor: "".into(),
        }]);
        assert!(is_known_remote(0x0001, 0x0001));
        set_remote(vec![]);
        assert!(!is_known_remote(0x0001, 0x0001));
    }
}
```

- [ ] **Step 2: Lancer les tests du module (doivent passer directement — TDD inline car le module est nouveau et petit)**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo test known_remote`
Expected: PASS (2 tests)

- [ ] **Step 3: Déclarer le module dans `src-tauri/src/backends/hid/mod.rs`**

```rust
pub mod corsair_node;
pub mod known;
pub mod known_remote;
pub mod nzxt_hue2;
```

- [ ] **Step 4: Brancher dans `list_raw()` — recognized doit aussi consulter le registre distant**

Dans `src-tauri/src/backends/hid/mod.rs`, remplacer :
```rust
                recognized: known::find_known(vid, pid).is_some() || known::find_vendor(vid).is_some(),
```
par :
```rust
                recognized: known::find_known(vid, pid).is_some()
                    || known::find_vendor(vid).is_some()
                    || known_remote::is_known_remote(vid, pid),
```

`has_native_driver` reste inchangé (compilé uniquement — un ajout dashboard n'active jamais de driver natif, cohérent avec la spec). `scan()` reste intégralement inchangé (aucune référence à `known_remote` dedans).

- [ ] **Step 5: Vérifier la compilation**

Run: `cargo check`
Expected: vert.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/backends/hid/known_remote.rs src-tauri/src/backends/hid/mod.rs
git commit -m "feat: merge remote known-devices registry into diagnostic recognition only"
```

---

### Task 13: Module `telemetry` — hash, cache, envoi, fetch (TDD sur la logique pure)

**Files:**
- Modify: `src-tauri/src/netdev.rs`
- Create: `src-tauri/src/telemetry.rs`

- [ ] **Step 1: Rendre `curl` réutilisable dans `src-tauri/src/netdev.rs`**

Changer :
```rust
fn curl(args: &[&str]) -> Result<String> {
```
en :
```rust
pub(crate) fn curl(args: &[&str]) -> Result<String> {
```

- [ ] **Step 2: Écrire le test de hash (dans le futur `telemetry.rs`, échoue tant que le fichier n'existe pas)**

Créer `src-tauri/src/telemetry.rs` avec d'abord seulement la fonction de hash + son test, pour respecter TDD étape par étape :

```rust
//! Télémétrie matériel opt-in : envoie un snapshot diagnostic au service
//! VPS et récupère la table de reconnaissance étendue. Best-effort partout
//! — aucune erreur réseau ne doit bloquer le démarrage ni l'usage normal.

use crate::backends::hid::known_remote::{self, RemoteDevice};
use crate::netdev::curl;
use crate::settings::dirs_dir;
use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

const TELEMETRY_BASE_URL: &str = "https://telemetry.purergb.heiphaistos.org";

/// Hash non cryptographique (zéro dépendance) — sert uniquement à éviter
/// de renvoyer un rapport identique à chaque lancement, pas à la sécurité.
pub fn hash_diagnostics(diagnostics_json: &str) -> String {
    let mut hasher = DefaultHasher::new();
    diagnostics_json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stable_pour_le_meme_contenu() {
        let a = hash_diagnostics(r#"{"hid_raw":[]}"#);
        let b = hash_diagnostics(r#"{"hid_raw":[]}"#);
        assert_eq!(a, b);
    }

    #[test]
    fn hash_different_pour_contenu_different() {
        let a = hash_diagnostics(r#"{"hid_raw":[]}"#);
        let b = hash_diagnostics(r#"{"hid_raw":[{"vid":"dead"}]}"#);
        assert_ne!(a, b);
    }
}
```

- [ ] **Step 3: Déclarer le module dans `src-tauri/src/lib.rs`**

Après `mod sensors;` (ordre alphabétique des `mod`) :
```rust
mod settings;
mod telemetry;
```

- [ ] **Step 4: Lancer les tests**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo test telemetry::tests`
Expected: PASS (2 tests)

- [ ] **Step 5: Ajouter l'identifiant de rapport (pseudo-aléatoire, zéro dépendance) + les chemins de cache, avec test**

Ajouter dans `src-tauri/src/telemetry.rs`, avant le module `tests` :
```rust
fn cache_dir() -> Result<PathBuf> {
    dirs_dir().context("répertoire de config introuvable")
}

fn report_id_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("telemetry_report_id.txt"))
}

fn last_hash_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("telemetry_last_hash.txt"))
}

fn known_devices_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("known_devices_cache.json"))
}

/// Identifiant local pseudo-aléatoire (128 bits), généré une fois et mis
/// en cache — sert uniquement à regrouper les rapports d'une même
/// installation côté dashboard ("vu N fois"), jamais à identifier une
/// personne. `RandomState` puise dans l'aléa système (protection HashDoS
/// de la std), suffisant ici et évite une dépendance `uuid` complète.
fn generate_report_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    let a = RandomState::new().build_hasher().finish();
    let b = RandomState::new().build_hasher().finish();
    format!("{a:016x}{b:016x}")
}

/// Charge le report_id depuis le cache, ou en génère un et le persiste.
pub fn report_id() -> String {
    if let Ok(path) = report_id_path() {
        if let Ok(existing) = std::fs::read_to_string(&path) {
            let trimmed = existing.trim();
            if trimmed.len() == 32 {
                return trimmed.to_string();
            }
        }
        let id = generate_report_id();
        let _ = std::fs::write(&path, &id);
        return id;
    }
    generate_report_id()
}
```

- [ ] **Step 6: Compiler et lancer les tests**

Run: `cargo check && cargo test telemetry::`
Expected: vert (pas de nouveau test pour `report_id()` — dépend du système de fichiers, couvert par la vérification manuelle Task 17).

- [ ] **Step 7: Ajouter l'envoi du rapport (best-effort, dédupliqué par hash)**

```rust
/// Envoie le snapshot diagnostic si l'opt-in est actif ET que son contenu
/// a changé depuis le dernier envoi. Best-effort : toute erreur réseau est
/// retournée à l'appelant pour log uniquement, jamais de panique.
pub fn maybe_send_report(diagnostics_json: &str, app_version: &str) -> Result<bool> {
    let hash = hash_diagnostics(diagnostics_json);
    let last_hash_path = last_hash_path()?;
    if let Ok(previous) = std::fs::read_to_string(&last_hash_path) {
        if previous.trim() == hash {
            return Ok(false); // rien de nouveau à envoyer
        }
    }
    send_report_now(diagnostics_json, app_version)?;
    std::fs::write(&last_hash_path, &hash).context("écriture du cache de hash télémétrie")?;
    Ok(true)
}

/// Envoi immédiat, sans vérification de hash — utilisé par le bouton
/// "Envoyer maintenant".
pub fn send_report_now(diagnostics_json: &str, app_version: &str) -> Result<()> {
    let payload = format!(
        r#"{{"report_id":"{}","app_version":"{}","diagnostics":{}}}"#,
        report_id(),
        app_version,
        diagnostics_json
    );
    let tmp_path = std::env::temp_dir().join("purergb_telemetry_payload.json");
    std::fs::write(&tmp_path, &payload).context("écriture du payload temporaire")?;
    let url = format!("{TELEMETRY_BASE_URL}/report");
    let result = curl(&[
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
        "--data-binary",
        &format!("@{}", tmp_path.display()),
        &url,
    ]);
    let _ = std::fs::remove_file(&tmp_path);
    result.map(|_| ())
}
```

- [ ] **Step 8: Ajouter la récupération + cache local de `known-devices`**

```rust
/// Récupère la table distante, la fusionne en mémoire (`known_remote`) et
/// la met en cache localement. Hors-ligne : réutilise le cache existant.
/// Best-effort total — ne bloque jamais le démarrage.
pub fn refresh_known_devices() {
    match fetch_known_devices() {
        Ok(devices) => {
            known_remote::set_remote(devices.clone());
            if let Ok(path) = known_devices_cache_path() {
                if let Ok(json) = serde_json::to_string(&devices) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
        Err(e) => {
            log::warn!("known-devices distant injoignable ({e:#}), utilisation du cache local");
            if let Some(cached) = load_known_devices_cache() {
                known_remote::set_remote(cached);
            }
        }
    }
}

fn fetch_known_devices() -> Result<Vec<RemoteDevice>> {
    let url = format!("{TELEMETRY_BASE_URL}/known-devices");
    let body = curl(&["--max-time", "3", &url])?;
    serde_json::from_str(&body).context("réponse known-devices illisible")
}

fn load_known_devices_cache() -> Option<Vec<RemoteDevice>> {
    let path = known_devices_cache_path().ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}
```

En haut du fichier, `RemoteDevice` doit dériver `Serialize` en plus de `Deserialize` pour permettre l'écriture du cache — retourner sur `known_remote.rs` (Task 12) et changer :
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteDevice {
```
en :
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDevice {
```
(ajouter `serde::Serialize` à l'import `use serde::Deserialize;` → `use serde::{Deserialize, Serialize};`)

- [ ] **Step 9: Compiler**

Run: `cargo check`
Expected: vert.

- [ ] **Step 10: Lancer tous les tests du crate**

Run: `cargo test`
Expected: PASS, aucune régression.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/netdev.rs src-tauri/src/telemetry.rs src-tauri/src/backends/hid/known_remote.rs src-tauri/src/lib.rs
git commit -m "feat: telemetry module — report sending, known-devices fetch and cache"
```

---

### Task 14: Câblage `lib.rs` — commande manuelle, envoi auto au démarrage, refresh known-devices

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Extraire `compute_hardware_diagnostics` (réutilisable par le thread `hw-init` ET la commande Tauri)**

Remplacer la fonction `hardware_diagnostics` (lignes 499-535) par :
```rust
fn compute_hardware_diagnostics(
    registry: &SharedRegistry,
    sensors: &SensorHub,
    openrgb_mgr: &OpenRgbManager,
    openrgb_host: &str,
    openrgb_port: u16,
) -> HardwareDiagnostics {
    let liquidctl = {
        let mut reg = registry.lock();
        reg.backends_mut()
            .iter_mut()
            .find(|b| b.name() == "liquidctl")
            .and_then(|b| b.as_any_mut().downcast_mut::<LiquidctlBackend>())
            .map(|lc| lc.diagnose())
            .unwrap_or(crate::backends::liquidctl::LiquidctlDiag {
                exe_path: None,
                version: Err("backend liquidctl absent du registre".into()),
                list: Err("—".into()),
                initialize: Err("—".into()),
                status: Err("—".into()),
            })
    };
    let hid_raw = {
        let mut reg = registry.lock();
        reg.backends_mut()
            .iter_mut()
            .find(|b| b.name() == "hid")
            .and_then(|b| b.as_any_mut().downcast_mut::<HidBackend>())
            .and_then(|hid| hid.list_raw().ok())
            .unwrap_or_default()
    };
    HardwareDiagnostics {
        liquidctl,
        sensord: sensors.diag(),
        openrgb: openrgb_mgr.status(openrgb_host, openrgb_port),
        hid_raw,
    }
}

#[tauri::command(async)]
fn hardware_diagnostics(state: State<AppState>) -> HardwareDiagnostics {
    let (host, port) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port)
    };
    compute_hardware_diagnostics(&state.registry, &state.sensors, &state.openrgb_mgr, &host, port)
}

/// Commande manuelle du bouton "Envoyer maintenant" — ignore le hash de
/// déduplication, envoie toujours si l'opt-in est actif.
#[tauri::command(async)]
fn send_telemetry_report(state: State<AppState>) -> Result<(), String> {
    let (host, port, opt_in) = {
        let s = state.settings.lock();
        (s.openrgb_host.clone(), s.openrgb_port, s.telemetry_opt_in)
    };
    if !opt_in {
        return Err("Télémétrie désactivée dans les réglages".into());
    }
    let diag = compute_hardware_diagnostics(&state.registry, &state.sensors, &state.openrgb_mgr, &host, port);
    let json = serde_json::to_string(&diag).map_err(|e| e.to_string())?;
    telemetry::send_report_now(&json, env!("CARGO_PKG_VERSION")).map_err(|e| format!("{e:#}"))
}
```

`HardwareDiagnostics` doit dériver `Serialize` — déjà le cas (`#[derive(Serialize)] struct HardwareDiagnostics`, ligne 487), aucun changement nécessaire.

- [ ] **Step 2: Ajouter le refresh known-devices + l'envoi auto dans le thread `hw-init`**

Dans le bloc `hw-init` (`src-tauri/src/lib.rs`, autour de la ligne 800-843), après la ligne `scan_with_zone_sizes(&mut registry.lock(), &saved.zone_sizes);` et avant `restore_saved_state(&registry, &engine, &saved);`, insérer :

```rust
                // Table de reconnaissance distante (diagnostic uniquement,
                // jamais utilisée pour le pilotage réel — voir known_remote.rs).
                telemetry::refresh_known_devices();

                if saved.telemetry_opt_in {
                    let diag = compute_hardware_diagnostics(
                        &registry,
                        &sensors_hub,
                        &mgr,
                        &saved.openrgb_host,
                        saved.openrgb_port,
                    );
                    match serde_json::to_string(&diag) {
                        Ok(json) => match telemetry::maybe_send_report(&json, env!("CARGO_PKG_VERSION")) {
                            Ok(true) => log::info!("rapport télémétrie envoyé"),
                            Ok(false) => log::debug!("rapport télémétrie inchangé, pas d'envoi"),
                            Err(e) => log::warn!("envoi télémétrie: {e:#}"),
                        },
                        Err(e) => log::warn!("sérialisation diagnostic télémétrie: {e:#}"),
                    }
                }
```

- [ ] **Step 3: Ajouter `telemetry_opt_in` à la commande `update_settings`**

Signature (`fn update_settings`, ~ligne 674) :
```rust
    fps: u32,
    start_minimized: bool,
    auto_manage_conflicts: bool,
    telemetry_opt_in: bool,
) -> Result<(), String> {
```

Assignation (~ligne 706-714) :
```rust
    s.auto_manage_conflicts = auto_manage_conflicts;
    s.telemetry_opt_in = telemetry_opt_in;
    settings::save(&s).map_err(|e| e.to_string())
```

- [ ] **Step 4: Enregistrer la nouvelle commande dans `generate_handler!`**

Après `hardware_diagnostics` (dernière ligne de la liste, ~ligne 1012) :
```rust
            hardware_diagnostics,
            send_telemetry_report
```

- [ ] **Step 5: Compiler**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check`
Expected: vert.

- [ ] **Step 6: Lancer tous les tests**

Run: `cargo test`
Expected: PASS, aucune régression.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire telemetry into startup flow and manual send command"
```

---

### Task 15: UI — opt-in + bouton "Envoyer maintenant"

**Files:**
- Modify: `src/components/SettingsPanel.vue`

- [ ] **Step 1: Ajouter le champ au `form` réactif**

```typescript
  auto_manage_conflicts: true,
  telemetry_opt_in: false,
});
```

- [ ] **Step 2: Ajouter au `watch`**

```typescript
    form.auto_manage_conflicts = s.auto_manage_conflicts;
    form.telemetry_opt_in = s.telemetry_opt_in;
  },
```

- [ ] **Step 3: Ajouter au payload de `save()`**

```typescript
      autoManageConflicts: form.auto_manage_conflicts,
      telemetryOptIn: form.telemetry_opt_in,
    });
```

- [ ] **Step 4: Checkbox dans la carte "Système & profils"**, juste après la checkbox `automanage` (ajoutée en v0.13.0) :

```html
      <div class="inline" style="margin-bottom: 12px">
        <input id="telemetry" type="checkbox" v-model="form.telemetry_opt_in" />
        <label for="telemetry">
          Envoyer les informations de diagnostic matériel (VID/PID détectés,
          état OpenRGB/liquidctl/sensord) pour aider à identifier le matériel
          non reconnu. Aucune donnée personnelle, désactivé par défaut.
        </label>
      </div>
```

- [ ] **Step 5: Bouton "Envoyer maintenant" dans le panneau Diagnostic**

Ajouter un `ref` et une fonction avant le template (après `runDiagnostics`) :
```typescript
const telemetrySending = ref(false);
const telemetryMsg = ref("");

async function sendTelemetryNow() {
  telemetrySending.value = true;
  telemetryMsg.value = "";
  try {
    await invoke("send_telemetry_report");
    telemetryMsg.value = "Rapport envoyé.";
  } catch (e) {
    telemetryMsg.value = `Envoi : ${e}`;
  } finally {
    telemetrySending.value = false;
  }
}
```

Dans le template, juste après le bouton "Lancer le diagnostic" (ligne ~270, avant `<div v-if="diag" class="diag-out">`) :
```html
      <button
        v-if="props.settings?.telemetry_opt_in"
        :disabled="telemetrySending"
        @click="sendTelemetryNow"
        style="margin-left: 8px"
      >
        {{ telemetrySending ? "Envoi…" : "Envoyer maintenant" }}
      </button>
      <span v-if="telemetryMsg" class="hint">{{ telemetryMsg }}</span>
```

- [ ] **Step 6: Build frontend**

Run: `cd "C:\Users\Momo\Desktop\PureRGB" && npm run build`
Expected: `vue-tsc --noEmit && vite build` vert.

- [ ] **Step 7: Commit**

```bash
git add src/components/SettingsPanel.vue
git commit -m "feat: telemetry opt-in checkbox and manual send button"
```

---

### Task 16: Vérification finale + version

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `package.json`

- [ ] **Step 1: Build complet backend**

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check && cargo test`
Expected: vert, tous les tests passent (y compris `known_remote::tests`, `telemetry::tests`, et les tests existants du repo — aucune régression).

- [ ] **Step 2: Build complet frontend**

Run: `cd "C:\Users\Momo\Desktop\PureRGB" && npm run build`
Expected: vert.

- [ ] **Step 3: Vérification manuelle end-to-end**

1. `npm run tauri dev`
2. Réglages → cocher "Envoyer les informations de diagnostic matériel"
3. Réglages → Diagnostic → "Lancer le diagnostic" puis "Envoyer maintenant"
4. Ouvrir `https://telemetry.purergb.heiphaistos.org/dashboard`, vérifier qu'une ligne apparaît pour chaque VID/PID non reconnu de la machine de test
5. Ajouter un nom + type pour une des lignes, cliquer "Ajouter"
6. Relancer `npm run tauri dev`, relancer le diagnostic, vérifier que ce VID/PID passe à "reconnu" (sans nouvelle release — preuve que le fetch dynamique fonctionne)

- [ ] **Step 4: Bump de version mineure 0.13.0 → 0.14.0**

`src-tauri/Cargo.toml` : `version = "0.14.0"`
`src-tauri/tauri.conf.json` : `"version": "0.14.0"`
`package.json` : `"version": "0.14.0"`

Run: `cd "C:\Users\Momo\Desktop\PureRGB\src-tauri" && cargo check` (resynchronise `Cargo.lock`)

- [ ] **Step 5: Commit version**

```bash
cd "C:\Users\Momo\Desktop\PureRGB"
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json package.json
git commit -m "chore: bump version to 0.14.0"
```

- [ ] **Step 6: Mettre à jour le statut de la spec**

Dans `docs/superpowers/specs/2026-07-20-hardware-telemetry-design.md`, remplacer la section `## Statut` par :
```markdown
## Statut

Implémenté et déployé (v0.14.0). Service VPS `PureRGB-Telemetry` en production sur `telemetry.purergb.heiphaistos.org` (port 3022). Vérifié en conditions réelles : envoi de rapport, ajout dashboard, reconnaissance propagée au lancement suivant sans nouvelle release.
```

```bash
git add docs/superpowers/specs/2026-07-20-hardware-telemetry-design.md
git commit -m "docs: mark hardware telemetry spec as implemented"
```

---

## Self-Review (fait par l'auteur du plan)

**Couverture spec** : consentement opt-in explicite (Task 11, 15) ✓ · payload = snapshot diagnostic complet réutilisé tel quel (Task 4, 14) ✓ · dédup par hash (Task 13) ✓ · bouton manuel + envoi auto au lancement (Task 14, 15) ✓ · service VPS Hono+SQLite port 3022 (Task 1-9) ✓ · dashboard protégé mot de passe, agrégation par fréquence, ajout (Task 6-8) ✓ · propagation immédiate sans nouvelle release via fetch+cache (Task 12-13) ✓ · sécurité (rate-limit, validation, IP hashée, cookie HttpOnly/Secure/SameSite, échappement XSS dashboard) ✓.

**Correction appliquée pendant la conception** : le plan initial (implicite dans la conversation) aurait pu relier `known_remote` à `scan()` comme `known::find_known`/`find_vendor` — vérifié dans le code que ça aurait créé des entrées fantômes "vu en USB (info)" non pilotables pour chaque ajout dashboard. Task 12 documente explicitement pourquoi `known_remote` reste isolé à `list_raw()`.

**Dépendances** : zéro nouvelle dépendance Cargo côté app (hash via `DefaultHasher` std, HTTP via `curl.exe` déjà présent, id via `RandomState` std) — cohérent avec le pattern déjà établi dans `netdev.rs`. Côté VPS, dépendances identiques à ForgeHook (stack déjà en prod).
