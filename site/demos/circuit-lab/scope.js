export function findTrigger(samples, level) {
  if (!samples || samples.length === 0) return -1;
  let wasBelow = samples[0] < level;
  for (let i = 1; i < samples.length; i++) {
    const isAbove = samples[i] >= level;
    if (wasBelow && isAbove) return i;
    wasBelow = !isAbove;
  }
  return -1;
}

export function toPixels(samples, settings, w, h) {
  if (!samples || samples.length === 0) return [];
  const { timeDiv, voltsDiv, triggerIndex = 0, sampleTime = 1e-4 } = settings;
  const pts = [];
  
  const yCenter = h / 2;
  const pixelsPerVolt = h / (8 * voltsDiv);
  const pixelsPerSec = w / (10 * timeDiv);
  const xPixelsPerSample = sampleTime * pixelsPerSec;
  
  for (let i = triggerIndex; i < samples.length; i++) {
    const x = (i - triggerIndex) * xPixelsPerSample;
    if (x > w) break;
    const y = yCenter - (samples[i] * pixelsPerVolt);
    pts.push(x, y);
  }
  return pts;
}
