export function startPhysicsLoop(
    getPhysics,
    executor,
    canvas,
    getState,
    readouts
) {
    let frameCount = 0;
    let lastFpsTime = performance.now();
    let isRunning = true;

    function loop() {
        if (!isRunning) return;
        
        const now = performance.now();
        frameCount++;
        if (now - lastFpsTime >= 1000) {
            readouts.updateFps(frameCount);
            frameCount = 0;
            lastFpsTime = now;
        }

        const physics = getPhysics();
        if (physics) {
            physics.step(1/60);
            
            readouts.update(physics.getTime(), physics.getStepTimeMs(), physics.getMomentum());
            readouts.updateEnergy(physics.getEnergy());

            const meshCount = physics.getMeshCount();
            for (let i = 0; i < meshCount; i++) {
                executor.uploadMesh(i, physics.getMeshVertices(i), physics.getMeshIndices(i));
            }

            const commands = physics.encode(canvas.width, canvas.height);
            executor.execute(commands, canvas.width, canvas.height);
            
            const state = getState();
            if (state.preset === 'double_pendulum') {
                const pos = physics.getPositions();
                const angle1 = Math.atan2(pos[7] - pos[4], pos[6] - pos[3]);
                const angle2 = Math.atan2(pos[16] - pos[13], pos[15] - pos[12]);
                let diff = Math.abs(angle1 - angle2);
                if (diff === 0) diff = 1e-16;
                const logDiff = Math.log10(diff).toFixed(2);
                readouts.updateChaosAngle(logDiff);
            } else {
                readouts.updateChaosAngle(null);
            }
        }
        requestAnimationFrame(loop);
    }
    loop();

    return {
        stop: () => { isRunning = false; }
    };
}
