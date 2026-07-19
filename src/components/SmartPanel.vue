<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import type { NetworkDevice, NetworkDeviceKind } from "../types";
import { NETWORK_KIND_LABELS } from "../types";

const emit = defineEmits<{
  toast: [msg: string];
  refresh: [];
}>();

const devices = ref<NetworkDevice[]>([]);
const busy = ref(false);
const pairing = ref(false);

const kind = ref<NetworkDeviceKind>("govee");
const ip = ref("");
const name = ref("");
const port = ref(16021);
const authToken = ref("");
const mac = ref("");
const entertainment = ref(false);
const musicMode = ref(true);
const multizone = ref(false);
const extendedMultizone = ref(false);
const numLeds = ref(30);
const startUniverse = ref(1);
const startChannel = ref(1);
const universeSize = ref(510);

const needsName = computed(() => ["lifx", "kasa", "e131"].includes(kind.value));

const hint = computed(() => {
  switch (kind.value) {
    case "hue":
      return "Entrez l'IP du pont puis « Appairer » : PureRGB récupère son identifiant. Appuyez ensuite sur le gros bouton du pont juste avant d'ajouter — OpenRGB finalise l'appairage au redémarrage.";
    case "nanoleaf":
      return "Maintenez le bouton power de l'appareil 5-7 s (les LED clignotent), puis cliquez « Obtenir le token » dans les 30 s.";
    case "yeelight":
      return "Activez « Contrôle LAN » dans l'app Yeelight (paramètres de l'ampoule).";
    case "govee":
      return "Activez « API LAN » dans l'app Govee Home (paramètres de l'appareil). Modèles compatibles LAN uniquement.";
    case "e131":
      return "Pour WLED : activez E1.31 dans Config → Sync Interfaces. 1 univers = 170 LEDs (universe size 510).";
    case "lifx":
      return "L'ampoule doit être sur le même réseau. IP visible dans l'app LIFX.";
    case "wiz":
      return "IP visible dans l'app Wiz (paramètres de l'ampoule).";
    case "kasa":
      return "Ampoules/bandeaux Kasa compatibles couleur uniquement.";
    default:
      return "L'appareil doit être sur le même réseau local que ce PC.";
  }
});

async function load() {
  devices.value = await invoke<NetworkDevice[]>("netdev_list");
}

async function pairHue() {
  pairing.value = true;
  try {
    mac.value = await invoke<string>("hue_pair", { ip: ip.value.trim() });
    emit("toast", `Pont trouvé (${mac.value}). Appuyez sur son bouton puis « Ajouter ».`);
  } catch (e) {
    emit("toast", `Appairage Hue : ${e}`);
  } finally {
    pairing.value = false;
  }
}

async function pairNanoleaf() {
  pairing.value = true;
  try {
    authToken.value = await invoke<string>("nanoleaf_pair", {
      ip: ip.value.trim(),
      port: port.value,
    });
    emit("toast", "Token Nanoleaf obtenu — cliquez « Ajouter ».");
  } catch (e) {
    emit("toast", `Appairage Nanoleaf : ${e}`);
  } finally {
    pairing.value = false;
  }
}

function buildDevice(): NetworkDevice {
  const base = { kind: kind.value, ip: ip.value.trim() };
  switch (kind.value) {
    case "hue":
      return { ...base, mac: mac.value, entertainment: entertainment.value };
    case "nanoleaf":
      return { ...base, port: port.value, auth_token: authToken.value };
    case "yeelight":
      return { ...base, music_mode: musicMode.value };
    case "lifx":
      return {
        ...base,
        name: name.value.trim(),
        multizone: multizone.value,
        extended_multizone: extendedMultizone.value,
      };
    case "kasa":
      return { ...base, name: name.value.trim() };
    case "e131":
      return {
        ...base,
        name: name.value.trim(),
        num_leds: numLeds.value,
        start_universe: startUniverse.value,
        start_channel: startChannel.value,
        universe_size: universeSize.value,
        keepalive_time: 0,
      };
    default:
      return base;
  }
}

async function add() {
  busy.value = true;
  try {
    await invoke("netdev_add", { device: buildDevice() });
    emit("toast", "Appareil ajouté — OpenRGB redémarré, scan en cours…");
    ip.value = "";
    name.value = "";
    mac.value = "";
    authToken.value = "";
    await load();
    emit("refresh");
  } catch (e) {
    emit("toast", `Ajout : ${e}`);
  } finally {
    busy.value = false;
  }
}

async function remove(index: number) {
  busy.value = true;
  try {
    await invoke("netdev_remove", { index });
    emit("toast", "Appareil retiré");
    await load();
    emit("refresh");
  } catch (e) {
    emit("toast", `Suppression : ${e}`);
  } finally {
    busy.value = false;
  }
}

function label(d: NetworkDevice): string {
  const extra = d.name ? ` « ${d.name} »` : "";
  return `${NETWORK_KIND_LABELS[d.kind]}${extra} — ${d.ip}`;
}

const canAdd = computed(() => {
  if (!ip.value.trim()) return false;
  if (kind.value === "hue" && !mac.value) return false;
  if (kind.value === "nanoleaf" && !authToken.value) return false;
  if (needsName.value && !name.value.trim()) return false;
  return true;
});

onMounted(load);
</script>

<template>
  <section class="smart-panel">
    <h2>Maison connectée</h2>
    <p class="sub">
      Ampoules, bandeaux Wi-Fi, panneaux lumineux… tout ce qu'un Google Home
      pilote en RGB. Les appareils ajoutés apparaissent dans l'onglet
      Éclairage et suivent les thèmes comme le reste du PC.
    </p>

    <div class="card">
      <h3>Ajouter un appareil</h3>
      <div class="form">
        <div class="row">
          <label>Type</label>
          <select v-model="kind">
            <option v-for="(l, k) in NETWORK_KIND_LABELS" :key="k" :value="k">
              {{ l }}
            </option>
          </select>
        </div>
        <div class="row">
          <label>Adresse IP</label>
          <input v-model="ip" placeholder="192.168.1.x" spellcheck="false" />
        </div>
        <div v-if="needsName" class="row">
          <label>Nom</label>
          <input v-model="name" placeholder="Bandeau salon" />
        </div>

        <template v-if="kind === 'hue'">
          <div class="row inline">
            <button :disabled="!ip || pairing" @click="pairHue">
              {{ pairing ? "Recherche…" : "Appairer le pont" }}
            </button>
            <span v-if="mac" class="ok-note">✓ pont identifié ({{ mac }})</span>
          </div>
          <div class="row inline">
            <input id="ent" type="checkbox" v-model="entertainment" />
            <label for="ent">Mode Entertainment (plus fluide, zones Hue)</label>
          </div>
        </template>

        <template v-if="kind === 'nanoleaf'">
          <div class="row">
            <label>Port</label>
            <input type="number" v-model.number="port" min="1" max="65535" />
          </div>
          <div class="row inline">
            <button :disabled="!ip || pairing" @click="pairNanoleaf">
              {{ pairing ? "Demande…" : "Obtenir le token" }}
            </button>
            <span v-if="authToken" class="ok-note">✓ token obtenu</span>
          </div>
        </template>

        <div v-if="kind === 'yeelight'" class="row inline">
          <input id="mm" type="checkbox" v-model="musicMode" />
          <label for="mm">Mode musique (mises à jour rapides, recommandé)</label>
        </div>

        <template v-if="kind === 'lifx'">
          <div class="row inline">
            <input id="mz" type="checkbox" v-model="multizone" />
            <label for="mz">Multizone (bandeau LIFX Z / Beam)</label>
          </div>
          <div class="row inline">
            <input id="emz" type="checkbox" v-model="extendedMultizone" />
            <label for="emz">Multizone étendu (firmware récent)</label>
          </div>
        </template>

        <template v-if="kind === 'e131'">
          <div class="grid4">
            <div class="row">
              <label>Nb LEDs</label>
              <input type="number" v-model.number="numLeds" min="1" max="4096" />
            </div>
            <div class="row">
              <label>Univers de départ</label>
              <input type="number" v-model.number="startUniverse" min="1" />
            </div>
            <div class="row">
              <label>Canal de départ</label>
              <input type="number" v-model.number="startChannel" min="1" max="512" />
            </div>
            <div class="row">
              <label>Taille d'univers</label>
              <input type="number" v-model.number="universeSize" min="3" max="512" />
            </div>
          </div>
        </template>

        <p class="hint">💡 {{ hint }}</p>

        <div class="actions">
          <button class="primary" :disabled="!canAdd || busy" @click="add">
            {{ busy ? "Application…" : "Ajouter" }}
          </button>
        </div>
      </div>
    </div>

    <div class="card">
      <h3>Appareils configurés ({{ devices.length }})</h3>
      <p v-if="devices.length === 0" class="sub">Aucun appareil réseau pour l'instant.</p>
      <ul>
        <li v-for="(d, i) in devices" :key="i">
          <span>{{ label(d) }}</span>
          <button class="danger" :disabled="busy" @click="remove(i)">Retirer</button>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.smart-panel {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
  max-width: 760px;
}

.smart-panel h2 {
  font-size: 19px;
}

.sub {
  color: var(--text-dim);
  font-size: 13px;
  margin-top: 4px;
}

.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 18px;
  margin-top: 18px;
}

.card h3 {
  font-size: 15px;
  margin-bottom: 12px;
}

.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.row label {
  display: block;
  font-size: 13px;
  color: var(--text-dim);
  margin-bottom: 5px;
}

.row input,
.row select {
  width: 100%;
  max-width: 320px;
}

.row.inline {
  display: flex;
  align-items: center;
  gap: 8px;
}

.row.inline label {
  margin: 0;
}

.grid4 {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 10px;
}

.hint {
  font-size: 12px;
  color: var(--text-dim);
  background: var(--accent-soft);
  border-radius: 8px;
  padding: 8px 12px;
}

.ok-note {
  color: var(--ok);
  font-size: 13px;
}

.actions {
  display: flex;
  gap: 10px;
}

ul {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 12px;
}

.danger {
  color: var(--warn);
}
</style>
