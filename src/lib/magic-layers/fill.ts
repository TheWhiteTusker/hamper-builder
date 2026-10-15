/**
 * Fills the holes left where text and objects were lifted off a picture.
 *
 * Works inward one ring at a time ("onion peel"): every hole pixel touching
 * known pixels takes their average, then becomes known for the next ring.
 *
 * ponytail: averaging fill. Clean on flat or gradient backdrops, smeary
 * inside big holes on textured photos. Upgrade path: an inpainting model
 * (e.g. MI-GAN ONNX via onnxruntime-web) for photographic backgrounds.
 */
export function fillHoles(data: Uint8ClampedArray, w: number, h: number, hole: Uint8Array) {
  const known = new Uint8Array(w * h);
  for (let i = 0; i < known.length; i++) known[i] = hole[i] ? 0 : 1;

  const queued = new Uint8Array(w * h);
  let ring: number[] = [];
  const enqueueAround = (i: number) => {
    const x = i % w;
    const y = (i - x) / w;
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
        const n = ny * w + nx;
        if (!known[n] && !queued[n]) {
          queued[n] = 1;
          ring.push(n);
        }
      }
    }
  };
  for (let i = 0; i < known.length; i++) if (known[i]) enqueueAround(i);

  const sums = new Float64Array(4);
  while (ring.length) {
    const current = ring;
    ring = [];
    const values = new Uint8ClampedArray(current.length * 4);
    current.forEach((i, k) => {
      const x = i % w;
      const y = (i - x) / w;
      sums.fill(0);
      let n = 0;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const nx = x + dx;
          const ny = y + dy;
          if (nx < 0 || ny < 0 || nx >= w || ny >= h || !known[ny * w + nx]) continue;
          const p = (ny * w + nx) * 4;
          for (let c = 0; c < 4; c++) sums[c] += data[p + c];
          n++;
        }
      }
      for (let c = 0; c < 4; c++) values[k * 4 + c] = sums[c] / n;
    });
    // Commit the whole ring at once, so each ring only borrows from the ones outside it.
    current.forEach((i, k) => {
      data.set(values.subarray(k * 4, k * 4 + 4), i * 4);
      known[i] = 1;
    });
    current.forEach(enqueueAround);
  }
}

/** Grows a 0/1 mask by `r` pixels in every direction (square brush), to cover anti-aliased edges. */
export function dilate(mask: Uint8Array, w: number, h: number, r: number): Uint8Array {
  if (r <= 0) return mask.slice();
  const across = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (!mask[y * w + x]) continue;
      for (let nx = Math.max(0, x - r); nx <= Math.min(w - 1, x + r); nx++) across[y * w + nx] = 1;
    }
  }
  const out = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (!across[y * w + x]) continue;
      for (let ny = Math.max(0, y - r); ny <= Math.min(h - 1, y + r); ny++) out[ny * w + x] = 1;
    }
  }
  return out;
}
