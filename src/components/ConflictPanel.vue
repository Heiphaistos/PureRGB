<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";
import type { ConflictReport, ConflictingSoftware } from "../types";

const props = defineProps<{
  conflicts: ConflictReport;
  openrgbManaged: boolean;
}>();

const emit = defineEmits<{ refresh: []; toast: [msg: string] }>();

const busy = ref<string | null>(null);

function isGuarded(family: string): boolean {
  return props.conflicts.guarded_families.includes(family);
}

async function toggleGuard(c: ConflictingSoftware) {
  busy.value = c.family;
  const next = !isGuarded(c.family);
  try {
    await invoke("conflict_guard_set", { family: c.family, enabled: next });
    emit(
      "toast",
      next
        ? `${c.name} : re-tué en continu tant que PureRGB tourne (relance forcée)`
        : `${c.name} : garde continue désactivée`,
    );
    emit("refresh");
  } catch (e) {
    emit("toast", `Garde : ${e}`);
  } finally {
    busy.value = null;
  }
}

function serviceSummary(c: ConflictingSoftware): string {
  if (c.services.length === 0) return "aucun service";
  return c.services
    .map((s) => `${s.display_name || s.name} (${s.state === "Running" ? "démarré" : "arrêté"}, ${s.start_mode})`)
    .join(", ");
}

async function act(c: ConflictingSoftware, action: "stop" | "disable" | "restore") {
  busy.value = c.family;
  try {
    if (action === "restore") {
      await invoke("conflict_restore", { family: c.family });
      emit("toast", `${c.name} réactivé`);
    } else {
      await invoke("conflict_stop", { family: c.family, disable: action === "disable" });
      emit(
        "toast",
        action === "disable"
          ? `${c.name} stoppé et désactivé au démarrage`
          : `${c.name} stoppé (reviendra au prochain démarrage)`,
      );
    }
    emit("refresh");
  } catch (e) {
    emit("toast", `Échec : ${e}`);
  } finally {
    busy.value = null;
  }
}

async function restartOpenRgb() {
  busy.value = "__openrgb";
  try {
    await invoke("openrgb_restart");
    emit("toast", "OpenRGB redémarré — nouvelle détection en cours");
    emit("refresh");
  } catch (e) {
    emit("toast", `OpenRGB : ${e}`);
  } finally {
    busy.value = null;
  }
}
</script>

<template>
  <section class="conflict-panel">
    <h2>Logiciels en conflit</h2>
    <p class="hint">
      Les logiciels constructeur (iCUE, CAM, Armoury Crate…) verrouillent l'accès
      au matériel : OpenRGB ne voit alors pas ces appareils. Stoppez-les ici, puis
      redémarrez OpenRGB pour relancer la détection. « Désactiver » empêche aussi
      leur relance au démarrage de Windows (réversible). Attention : leurs
      fonctions propres (macros, écrans LCD, mises à jour) seront indisponibles
      tant qu'ils sont stoppés.
    </p>

    <p v-if="props.conflicts.conflicts.length === 0" class="empty">
      Aucun logiciel RGB constructeur détecté (ni processus, ni service). ✔
    </p>

    <article v-for="c in props.conflicts.conflicts" :key="c.family" class="card">
      <header>
        <span class="status-dot" :class="{ active: c.active }"></span>
        <strong>{{ c.name }}</strong>
        <span class="affects" v-if="c.affects.length">
          matériel {{ c.affects.join(", ") }}
        </span>
      </header>
      <p class="detail" v-if="c.processes.length">
        Processus actifs : {{ c.processes.join(", ") }}
      </p>
      <p class="detail">Services : {{ serviceSummary(c) }}</p>
      <div class="actions">
        <button :disabled="busy !== null || !c.active" @click="act(c, 'stop')">
          {{ busy === c.family ? "..." : "Stopper" }}
        </button>
        <button
          class="warn"
          :disabled="busy !== null"
          @click="act(c, 'disable')"
          title="Stoppe et empêche la relance au démarrage de Windows"
        >
          Stopper + désactiver
        </button>
        <button :disabled="busy !== null" @click="act(c, 'restore')">
          Réactiver
        </button>
        <button
          class="guard"
          :class="{ on: isGuarded(c.family) }"
          :disabled="busy !== null"
          @click="toggleGuard(c)"
          title="Re-tue le processus toutes les 12 s tant que PureRGB tourne — pour les logiciels qui se relancent seuls malgré service désactivé"
        >
          {{ isGuarded(c.family) ? "🛡️ Garde active" : "Garder désactivé" }}
        </button>
      </div>
      <p v-if="isGuarded(c.family)" class="guard-note">
        🛡️ PureRGB re-tue {{ c.name }} en continu — s'il revient quand même,
        redémarrez Windows après « Stopper + désactiver ».
      </p>
    </article>

    <div class="rescan" v-if="props.conflicts.conflicts.length > 0">
      <button
        class="primary"
        :disabled="busy !== null || !props.openrgbManaged"
        :title="props.openrgbManaged ? '' : 'OpenRGB non géré par PureRGB — redémarrez-le manuellement'"
        @click="restartOpenRgb"
      >
        {{ busy === "__openrgb" ? "Redémarrage..." : "↻ Redémarrer OpenRGB + re-scanner" }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.conflict-panel {
  flex: 1;
  overflow-y: auto;
  padding: 22px;
  max-width: 760px;
}

h2 {
  font-size: 16px;
  margin-bottom: 8px;
}

.hint {
  font-size: 13px;
  color: var(--text-dim);
  line-height: 1.6;
  margin-bottom: 18px;
}

.empty {
  color: var(--ok);
  font-size: 14px;
}

.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
  margin-bottom: 12px;
}

.card header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--text-dim);
}

.status-dot.active {
  background: var(--warn);
  box-shadow: 0 0 8px var(--warn);
}

.affects {
  font-size: 12px;
  color: var(--text-dim);
}

.detail {
  font-size: 12px;
  color: var(--text-dim);
  margin: 3px 0;
  word-break: break-word;
}

.actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.actions .warn {
  border-color: var(--warn);
  color: var(--warn);
}

.actions .guard.on {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}

.guard-note {
  margin-top: 8px;
  font-size: 12px;
  color: var(--accent);
}

.rescan {
  margin-top: 18px;
}
</style>
