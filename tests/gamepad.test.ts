import test from "node:test";
import assert from "node:assert/strict";
import { buttonName, drift, padIdentity, stick } from "../src/gamepad.ts";

test("zone morte circulaire", () => {
  assert.equal(stick(0.05, 0.05).inDeadzone, true);
  assert.equal(stick(0.2, 0).inDeadzone, false);
  assert.equal(stick(1, 1).magnitude, 1);
});

test("dérive", () => {
  assert.equal(drift([[0, 0.01], [0.02, 0]]).drifting, false);
  assert.equal(drift([[0.08, 0.02]]).drifting, true);
  assert.equal(drift([]).max, 0);
});

test("noms de boutons", () => {
  assert.equal(buttonName(0), "A / Croix");
  assert.equal(buttonName(40), "Bouton 40");
});

test("identité des manettes d'après l'id du navigateur", () => {
  const ds = padIdentity("Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)");
  assert.deepEqual([ds.vendor, ds.usb], ["Sony", "054c:0ce6"]);
  assert.equal(padIdentity("Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)").usb, "057e:2009");
  assert.equal(padIdentity("054c-09cc-Wireless Controller").usb, "054c:09cc");
  const x = padIdentity("Xbox 360 Controller (XInput STANDARD GAMEPAD)");
  assert.equal(x.vendor, "Microsoft");
  assert.equal(x.usb, null);
  assert.match(x.label, /XInput/);
  assert.equal(padIdentity("Truc inconnu").usb, null);
});
