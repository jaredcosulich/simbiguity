import { findTrigger, toPixels } from '../scope.js';

export class ScopeView {
  constructor(canvas) {
    this.ctx = canvas.getContext("2d");
    this.canvas = canvas;
    this.lastTrace1 = [];
    this.lastTrace2 = [];
  }
  
  draw(samples1, samples2, settings) {
    const { ctx, canvas } = this;
    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);
    
    ctx.strokeStyle = "rgba(255,255,255,0.2)";
    ctx.beginPath();
    for (let i = 1; i < 10; i++) {
      const x = (i / 10) * w;
      ctx.moveTo(x, 0); ctx.lineTo(x, h);
    }
    for (let i = 1; i < 8; i++) {
      const y = (i / 8) * h;
      ctx.moveTo(0, y); ctx.lineTo(w, y);
    }
    ctx.stroke();
    
    ctx.strokeStyle = "rgba(255,255,255,0.5)";
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();
    
    let triggerIdx = findTrigger(samples1, settings.triggerLevel || 0);
    if (triggerIdx === -1 && this.lastTrace1.length > 0) {
      // Hold last trace
    } else {
      if (triggerIdx === -1) triggerIdx = 0;
      this.lastTrace1 = toPixels(samples1, { ...settings, triggerIndex: triggerIdx }, w, h);
      this.lastTrace2 = toPixels(samples2, { ...settings, triggerIndex: triggerIdx }, w, h);
    }
    
    if (this.lastTrace1 && this.lastTrace1.length > 0) {
      ctx.strokeStyle = "cyan";
      ctx.beginPath();
      for (let i = 0; i < this.lastTrace1.length; i += 2) {
        if (i === 0) ctx.moveTo(this.lastTrace1[i], this.lastTrace1[i+1]);
        else ctx.lineTo(this.lastTrace1[i], this.lastTrace1[i+1]);
      }
      ctx.stroke();
    }
    
    if (this.lastTrace2 && this.lastTrace2.length > 0) {
      ctx.strokeStyle = "magenta";
      ctx.beginPath();
      for (let i = 0; i < this.lastTrace2.length; i += 2) {
        if (i === 0) ctx.moveTo(this.lastTrace2[i], this.lastTrace2[i+1]);
        else ctx.lineTo(this.lastTrace2[i], this.lastTrace2[i+1]);
      }
      ctx.stroke();
    }
  }
}