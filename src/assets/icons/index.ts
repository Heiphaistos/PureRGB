import type { Component } from "vue";
import motherboard from "./motherboard.vue";
import dram from "./dram.vue";
import gpu from "./gpu.vue";
import cooler from "./cooler.vue";
import led_strip from "./led_strip.vue";
import keyboard from "./keyboard.vue";
import mouse from "./mouse.vue";
import mousemat from "./mousemat.vue";
import headset from "./headset.vue";
import headset_stand from "./headset_stand.vue";
import gamepad from "./gamepad.vue";
import light from "./light.vue";
import speaker from "./speaker.vue";
import virtual_ from "./virtual.vue";
import storage from "./storage.vue";
import caseIcon from "./case.vue";
import microphone from "./microphone.vue";
import accessory from "./accessory.vue";
import keypad from "./keypad.vue";
import fan from "./fan.vue";
import hub from "./hub.vue";
import aio from "./aio.vue";
import unknown from "./unknown.vue";

export const DEVICE_ICONS: Record<string, Component> = {
  motherboard,
  dram,
  gpu,
  cooler,
  led_strip,
  keyboard,
  mouse,
  mousemat,
  headset,
  headset_stand,
  gamepad,
  light,
  speaker,
  virtual: virtual_,
  storage,
  case: caseIcon,
  microphone,
  accessory,
  keypad,
  fan,
  hub,
  aio,
  unknown,
};
