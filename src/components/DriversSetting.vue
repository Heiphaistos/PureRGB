<script setup lang="ts">
/** Pilotage des appareils : un seul endroit, un seul principe (PureRGB choisit le meilleur pilote tout seul). */
defineProps<{
  form: {
    openrgb_host: string;
    openrgb_port: number;
    auto_start_openrgb: boolean;
    native_drivers_enabled: boolean;
    smbus_writes_enabled: boolean;
    auto_manage_conflicts: boolean;
  };
}>();
</script>

<template>
  <div class="card">
    <h3>Pilotage des appareils</h3>
    <p class="hint">
      PureRGB choisit tout seul le meilleur pilote pour chaque appareil : son pilote intégré quand il existe, sinon OpenRGB
      (inclus, lancé en arrière-plan). Si le pilote intégré échoue, OpenRGB prend le relais automatiquement, sans doublon
      dans la liste ni conflit. Vous n'avez rien d'autre à régler.
    </p>

    <div class="inline">
      <input id="native" type="checkbox" v-model="form.native_drivers_enabled" />
      <label for="native">
        Utiliser les pilotes intégrés (expérimental) : cartes mères ASUS / ASRock / MSI / Gigabyte (USB), hubs Corsair, NZXT,
        Lian Li et Thermaltake, Corsair Commander Core, Cooler Master ARGB, claviers Logitech G410/G810/G910, SteelSeries Apex Pro TKL et Aerox 3, souris Razer, Logitech G203/G303/G403/G Pro, SteelSeries Rival 600 et Roccat Kone Pro, Wooting, HyperX, mémoire Corsair. Sans cette case, tout passe par OpenRGB.
      </label>
    </div>
    <div class="inline sub">
      <input id="smbusw" type="checkbox" v-model="form.smbus_writes_enabled" :disabled="!form.native_drivers_enabled" />
      <label for="smbusw">
        Autoriser l'écriture sur la mémoire RAM (bus SMBus). Sans cela, la mémoire est seulement détectée en lecture. Faites
        d'abord le « Test de compatibilité et de sécurité ».
      </label>
    </div>
    <div class="inline">
      <input id="automanage" type="checkbox" v-model="form.auto_manage_conflicts" />
      <label for="automanage">
        Fermer automatiquement les logiciels constructeur qui verrouillent les appareils (iCUE, CAM, Armoury Crate, TRCC…) au
        lancement, et les relancer à la fermeture.
      </label>
    </div>

    <details class="adv">
      <summary>Avancé : serveur OpenRGB</summary>
      <p class="hint">
        OpenRGB est démarré automatiquement. Si le port est déjà pris par un autre programme, PureRGB en choisit un libre tout
        seul. Un OpenRGB déjà lancé par vous est réutilisé tel quel.
      </p>
      <div class="inline">
        <input id="autostart" type="checkbox" v-model="form.auto_start_openrgb" />
        <label for="autostart">Démarrer OpenRGB automatiquement avec PureRGB</label>
      </div>
      <div class="grid2">
        <div>
          <label>Hôte</label>
          <input type="text" v-model="form.openrgb_host" maxlength="253" />
        </div>
        <div>
          <label>Port</label>
          <input type="number" v-model.number="form.openrgb_port" min="1" max="65535" />
        </div>
      </div>
    </details>
  </div>
</template>

<style scoped>
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 18px;
  margin-bottom: 14px;
}
h3 {
  font-size: 14px;
  margin-bottom: 8px;
}
.hint {
  color: var(--text-dim);
  font-size: 12px;
  margin-bottom: 12px;
}
.inline {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  margin-bottom: 12px;
  font-size: 13px;
}
.inline.sub {
  margin-left: 22px;
}
.adv {
  margin-top: 6px;
  font-size: 13px;
}
.adv summary {
  cursor: pointer;
  color: var(--text-dim);
  margin-bottom: 8px;
}
.grid2 {
  display: grid;
  grid-template-columns: 1fr 120px;
  gap: 12px;
}
</style>
