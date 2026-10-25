<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { ref, watch } from "vue";
import { cleanNote, NOTE_MAX, VERDICTS, type Verdict } from "../communityFeedback";
import { errMsg } from "../screens/api";

const props = defineProps<{ vid: string; pid: string; manufacturer: string; product: string }>();

const open = ref(false);
const verdict = ref<Verdict>("works");
const note = ref("");
const consent = ref(false);
const preview = ref("");
const busy = ref(false);
const msg = ref("");
const sent = ref(false);

const args = () => ({
  vid: props.vid,
  pid: props.pid,
  manufacturer: props.manufacturer,
  product: props.product,
  verdict: verdict.value,
  note: note.value,
});

/** Aperçu = JSON exact produit par le même code Rust que l'envoi. */
async function refreshPreview(): Promise<void> {
  try {
    preview.value = await invoke<string>("device_feedback_preview", args());
  } catch (e) {
    preview.value = errMsg(e);
  }
}
watch([open, verdict, note], () => open.value && refreshPreview());

function show(): void {
  consent.value = false;
  sent.value = false;
  msg.value = "";
  open.value = true;
}

async function send(): Promise<void> {
  busy.value = true;
  msg.value = "";
  try {
    await invoke("device_feedback_send", { ...args(), consent: consent.value });
    sent.value = true;
    msg.value = "Merci ! Fiche envoyée.";
  } catch (e) {
    msg.value = `Envoi impossible : ${errMsg(e)}`;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <button type="button" class="fb-open" title="Dire si cet appareil fonctionne, pour aider la communauté" @click.stop="show">
    Signaler / confirmer
  </button>
  <Teleport to="body">
    <div v-if="open" class="fb-modal" @click.self="open = false">
      <div class="fb-inner" role="dialog" aria-modal="true">
        <h4>Signaler l'appareil {{ vid }}:{{ pid }}</h4>
        <p class="hint">
          Cet appareil fonctionne-t-il avec PureRGB ? Votre réponse alimente le catalogue communautaire.
        </p>
        <label v-for="v in VERDICTS" :key="v.value" class="fb-radio">
          <input v-model="verdict" type="radio" :value="v.value" /> {{ v.label }}
        </label>
        <label class="fb-note">
          Note facultative ({{ cleanNote(note).length }}/{{ NOTE_MAX }})
          <textarea v-model="note" :maxlength="NOTE_MAX" rows="3" placeholder="Ex. : testé avec le logiciel du constructeur fermé"></textarea>
        </label>
        <p class="hint">Contenu exact qui sera envoyé (rien d'autre : ni nom d'utilisateur, ni chemin, ni numéro de série, ni capture USB) :</p>
        <pre class="fb-preview">{{ preview }}</pre>
        <label class="fb-radio">
          <input v-model="consent" type="checkbox" :disabled="sent" /> J'accepte d'envoyer ce contenu au serveur de PureRGB.
        </label>
        <p v-if="msg" class="hint">{{ msg }}</p>
        <div class="fb-actions">
          <button type="button" :disabled="!consent || busy || sent" @click="send">{{ busy ? "Envoi…" : "Envoyer" }}</button>
          <button type="button" @click="open = false">{{ sent ? "Fermer" : "Annuler" }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.fb-open { font-size: 12px; }
.fb-modal { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.6); display: flex; align-items: center; justify-content: center; z-index: 100; }
.fb-inner { background: var(--bg-panel, #1a1a1a); padding: 20px; border-radius: 8px; max-width: 520px; width: 92%; max-height: 90vh; overflow: auto; }
.fb-radio { display: block; margin: 4px 0; }
.fb-note { display: block; margin: 10px 0; }
.fb-note textarea { width: 100%; box-sizing: border-box; }
.fb-preview { background: rgba(0, 0, 0, 0.35); padding: 8px; border-radius: 4px; font-size: 12px; white-space: pre-wrap; word-break: break-all; max-height: 180px; overflow: auto; }
.fb-actions { display: flex; gap: 10px; margin-top: 12px; }
</style>
