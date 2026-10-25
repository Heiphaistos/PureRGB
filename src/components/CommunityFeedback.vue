<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import { summaryLabel, type CommunityEntry } from "../communityFeedback";
import { errMsg } from "../screens/api";

interface CommunityList {
  entries: CommunityEntry[];
  offline: boolean;
}

const list = ref<CommunityList | null>(null);
const busy = ref(false);
const error = ref("");
const filter = ref("");

const rows = computed(() => {
  const q = filter.value.trim().toLowerCase();
  return (list.value?.entries ?? []).filter((e) => !q || `${e.vid}:${e.pid}`.includes(q));
});

async function load(): Promise<void> {
  busy.value = true;
  error.value = "";
  try {
    list.value = await invoke<CommunityList>("community_feedback_list");
  } catch (e) {
    error.value = errMsg(e);
  } finally {
    busy.value = false;
  }
}
onMounted(load);
</script>

<template>
  <div class="community">
    <h4>
      Retours de la communauté
      <button type="button" :disabled="busy" @click="load">{{ busy ? "…" : "Actualiser" }}</button>
    </h4>
    <p class="hint">
      Comptes anonymes par appareil (lecture seule).
      <template v-if="list?.offline"> Serveur injoignable : données du dernier chargement.</template>
    </p>
    <p v-if="error" class="hint">{{ error }}</p>
    <input v-model="filter" type="text" placeholder="Filtrer par VID:PID" maxlength="9" />
    <table v-if="rows.length" class="diag-table hid-table">
      <tr><th>VID:PID</th><th>Retours</th></tr>
      <tr v-for="e in rows" :key="`${e.vid}:${e.pid}`">
        <td>{{ e.vid }}:{{ e.pid }}</td>
        <td>{{ summaryLabel(e) }}</td>
      </tr>
    </table>
    <p v-else-if="!busy" class="hint">Aucun retour communautaire à afficher.</p>
  </div>
</template>
