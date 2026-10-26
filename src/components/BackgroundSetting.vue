<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { errMsg } from "../screens/api";

interface Modules {
  screens: boolean;
  lighting: boolean;
  fans: boolean;
  gamepad: boolean;
}

const m = reactive<Modules>({ screens: false, lighting: false, fans: false, gamepad: false });
const error = ref("");
const items: { key: keyof Modules; label: string; hint: string }[] = [
  { key: "screens", label: "Écrans (design, thème, vidéo)", hint: "Les mini-écrans gardent leur affichage." },
  { key: "lighting", label: "Éclairage appliqué", hint: "Couleurs, effets et thèmes déjà réglés (OpenRGB et pilotes natifs)." },
  { key: "fans", label: "Ventilateurs", hint: "Seulement les courbes et réglages manuels déjà actifs. Jamais de réglage automatique." },
  { key: "gamepad", label: "Mode manette", hint: "Remappage par manette virtuelle, si activé." },
];

onMounted(async () => {
  try {
    Object.assign(m, await invoke<Modules>("background_get"));
  } catch {
    /* hors application (aperçu navigateur) : cases non enregistrées */
  }
});

async function save(): Promise<void> {
  error.value = "";
  try {
    await invoke("background_set", { modules: { ...m } });
  } catch (e) {
    error.value = errMsg(e);
  }
}
</script>

<template>
  <div class="card wide">
    <h3>Rester actif quand PureRGB est fermé</h3>
    <p class="hint">
      Au « Quitter » du menu de la zone de notification, PureRGB peut laisser un petit processus sans fenêtre
      (même programme, sans interface) qui ne garde que ce que vous cochez et qui est déjà en service. « Quitter
      complètement » arrête tout. Ouvrir PureRGB à nouveau reprend la main. Le mode jeu s'applique aussi à ce
      processus, comme dans l'application complète.
    </p>
    <div v-for="it in items" :key="it.key" class="inline">
      <input :id="'bg-' + it.key" v-model="m[it.key]" type="checkbox" @change="save" />
      <label :for="'bg-' + it.key">{{ it.label }} <small>{{ it.hint }}</small></label>
    </div>
    <p v-if="error" class="err">{{ error }}</p>
  </div>
</template>
