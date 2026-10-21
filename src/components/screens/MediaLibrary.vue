<script setup lang="ts">
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { onBeforeUnmount, onMounted, ref } from "vue";
import { demo, errMsg, mediaDelete, mediaImport, mediaImportFile, MEDIA_FILTERS, pickFiles } from "../../screens/api.ts";
import { loadMedia, media } from "../../screens/store.ts";
import { forgetThumb } from "../../screens/thumbs.ts";
import type { MediaItem } from "../../screens/types.ts";
import MediaThumb from "./MediaThumb.vue";
import "./screens.css";

const emit = defineEmits<{ use: [mediaId: string, as: "background" | "image"]; toast: [msg: string] }>();

const dragging = ref(false);
const busy = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);
let unlisten: (() => void) | undefined;

const EXT = /\.(png|jpe?g|bmp|webp|gif|mp4|m4v|webm|mkv|avi|mov|wmv)$/i;

async function importPaths(paths: string[]): Promise<void> {
  const ok = paths.filter((p) => EXT.test(p));
  if (ok.length < paths.length) emit("toast", `${paths.length - ok.length} fichier(s) ignoré(s) : format non supporté`);
  busy.value = true;
  try {
    for (const p of ok) await mediaImport(p);
    await loadMedia();
    if (ok.length) emit("toast", `${ok.length} média(s) importé(s)`);
  } catch (e) {
    emit("toast", `Import : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}

async function importFiles(files: File[]): Promise<void> {
  busy.value = true;
  try {
    for (const f of files) {
      if (!f.type.startsWith("image/") && !f.type.startsWith("video/")) {
        emit("toast", `${f.name} : format non accepté en mode démonstration`);
        continue;
      }
      await mediaImportFile(f);
    }
    await loadMedia();
  } catch (e) {
    emit("toast", `Import : ${errMsg(e)}`);
  } finally {
    busy.value = false;
  }
}

async function pick(): Promise<void> {
  if (demo.value) {
    fileInput.value?.click();
    return;
  }
  try {
    await importPaths(await pickFiles("Importer des médias", MEDIA_FILTERS));
  } catch (e) {
    emit("toast", errMsg(e));
  }
}

function onDrop(e: DragEvent): void {
  dragging.value = false;
  if (demo.value) void importFiles([...(e.dataTransfer?.files ?? [])]);
}

function onInput(e: Event): void {
  const t = e.target as HTMLInputElement;
  void importFiles([...(t.files ?? [])]);
  t.value = "";
}

async function remove(m: MediaItem): Promise<void> {
  if (!window.confirm(`Supprimer « ${m.name} » de la médiathèque ? Les designs qui l'utilisent n'auront plus d'image.`)) return;
  try {
    await mediaDelete(m.id);
    forgetThumb(`media:${m.id}`);
    await loadMedia();
  } catch (e) {
    emit("toast", errMsg(e));
  }
}

function info(m: MediaItem): string {
  const parts = [`${m.width}×${m.height}`];
  if (m.frames > 1) parts.push(`${m.frames} images`);
  if (m.duration_ms > 0) parts.push(`${(m.duration_ms / 1000).toFixed(1)} s`);
  return parts.join(" · ");
}

const KIND: Record<string, string> = { image: "Image", gif: "GIF", video: "Vidéo", font: "Police" };

onMounted(async () => {
  try {
    await loadMedia();
  } catch (e) {
    emit("toast", `Médias : ${errMsg(e)}`);
  }
  if (!demo.value) {
    try {
      unlisten = await getCurrentWebview().onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === "over") dragging.value = true;
        else if (p.type === "drop") {
          dragging.value = false;
          void importPaths(p.paths);
        } else dragging.value = false;
      });
    } catch {
      // Glisser-déposer natif indisponible : le bouton Importer reste utilisable.
    }
  }
});
onBeforeUnmount(() => unlisten?.());
</script>

<template>
  <div
    class="lib"
    :class="{ drag: dragging }"
    @dragover.prevent="dragging = true"
    @dragleave="dragging = false"
    @drop.prevent="onDrop"
  >
    <div class="sc-row">
      <div class="sc-grow">
        <h3>Médiathèque</h3>
        <p class="sc-sub">Images, GIF animés et vidéos. Glissez-déposez des fichiers ici ou utilisez le bouton.</p>
      </div>
      <button class="primary" :disabled="busy" @click="pick">{{ busy ? "Import…" : "Importer" }}</button>
      <input ref="fileInput" type="file" accept="image/*,video/*" multiple hidden @change="onInput" />
    </div>

    <div v-if="media.length" class="grid">
      <article v-for="m in media" :key="m.id" class="sc-card item">
        <MediaThumb :item="m" />
        <div class="meta">
          <strong class="nm" :title="m.name">{{ m.name }}</strong>
          <span class="dim"><span class="sc-badge">{{ KIND[m.kind] ?? m.kind }}</span> {{ info(m) }}</span>
        </div>
        <div class="acts">
          <button class="sm" title="Utiliser comme fond du design en cours d'édition" @click="emit('use', m.id, 'background')">Fond</button>
          <button class="sm" title="Ajouter un widget Image au design" @click="emit('use', m.id, 'image')">Widget</button>
          <button class="sm danger" @click="remove(m)">Supprimer</button>
        </div>
      </article>
    </div>
    <div v-else class="drop sc-card">
      <p>Aucun média pour l'instant.</p>
      <p class="sc-sub">Déposez une image, un GIF ou une vidéo ici pour l'utiliser en fond ou comme logo.</p>
    </div>
    <div v-if="dragging" class="veil">Déposez pour importer</div>
  </div>
</template>

<style scoped>
.lib {
  position: relative;
  overflow-y: auto;
  padding: 18px 26px;
}

h3 {
  font-size: 16px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 14px;
  margin-top: 16px;
}

.item {
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.nm {
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: block;
}

.dim {
  color: var(--text-dim);
  font-size: 11px;
}

.acts {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.drop {
  margin-top: 18px;
  padding: 40px;
  text-align: center;
  border-style: dashed;
}

.veil {
  position: absolute;
  inset: 8px;
  border: 2px dashed var(--accent);
  border-radius: var(--radius);
  background: rgba(255, 80, 0, 0.1);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
  font-weight: 600;
  pointer-events: none;
}
</style>
