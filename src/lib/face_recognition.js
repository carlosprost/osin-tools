import * as faceapi from "face-api.js";
import { invoke } from "@tauri-apps/api/core";

// Configuration
const MODEL_URL = "/models";

// State
let loadedNets = {
  ssd: false,
  tiny: false,
  landmarks: false,
  recognition: false,
};
let labeledFaceDescriptors = []; // To store known faces

/**
 * Load models from public/models
 * @param {boolean} useTiny - Whether to load Tiny Face Detector models
 */
export async function loadModels(useTiny = false) {
  try {
    const msgStart = `[BIOMETRÍA] Verificando modelos (${useTiny ? "Rápido" : "Preciso"})...`;
    console.log(msgStart);
    await invoke("log_info", { message: msgStart });

    // Forzar CPU para estabilidad total en video integrado
    if (faceapi.tf.getBackend() !== "cpu") {
      await faceapi.tf.setBackend("cpu");
      await invoke("log_info", {
        message: `[BIOMETRÍA] Backend CPU activado.`,
      });
    }

    if (useTiny && !loadedNets.tiny) {
      await faceapi.nets.tinyFaceDetector.loadFromUri(MODEL_URL);
      loadedNets.tiny = true;

      // Warm-up real: Detección en canvas vacío
      const warmupCanvas = document.createElement("canvas");
      warmupCanvas.width = 128;
      warmupCanvas.height = 128;
      await faceapi.detectSingleFace(
        warmupCanvas,
        new faceapi.TinyFaceDetectorOptions({ inputSize: 128 }),
      );
      await invoke("log_info", {
        message: `[BIOMETRÍA] Warm-up Tiny completado.`,
      });
    }

    if (!useTiny && !loadedNets.ssd) {
      await faceapi.nets.ssdMobilenetv1.loadFromUri(MODEL_URL);
      loadedNets.ssd = true;
    }

    if (!loadedNets.landmarks) {
      await faceapi.nets.faceLandmark68Net.loadFromUri(MODEL_URL);
      loadedNets.landmarks = true;
    }

    if (!loadedNets.recognition) {
      await faceapi.nets.faceRecognitionNet.loadFromUri(MODEL_URL);
      loadedNets.recognition = true;
    }

    await invoke("log_info", {
      message: "[BIOMETRÍA] Modelos listos y pre-calentados.",
    });
  } catch (e) {
    console.error("Error cargando modelos face-api:", e);
    throw e;
  }
}

/**
 * Detect face and extract 128-float descriptor
 * @param {HTMLImageElement | HTMLVideoElement | HTMLCanvasElement} input
 * @param {boolean} useTiny - Whether to use the Tiny Face Detector
 */
export async function getFaceDescriptor(input, useTiny = false) {
  try {
    await loadModels(useTiny);

    let processedInput = input;
    const MAX_DIM = 600; // Resolución equilibrada para detección nítida
    if (input.width > MAX_DIM || input.height > MAX_DIM) {
      const scale = Math.min(MAX_DIM / input.width, MAX_DIM / input.height);
      const newWidth = Math.round(input.width * scale);
      const newHeight = Math.round(input.height * scale);

      const canvas = document.createElement("canvas");
      canvas.width = newWidth;
      canvas.height = newHeight;
      const ctx = canvas.getContext("2d");
      ctx.drawImage(input, 0, 0, newWidth, newHeight);
      processedInput = canvas;
    }

    // --- PRUEBA DE VIDA TFJS ---
    try {
      const testTensor = faceapi.tf.tensor1d([1, 2, 3]);
      testTensor.dispose(); // Solo verificamos que el engine no esté bloqueado
    } catch (tfErr) {
      await invoke("log_info", {
        message: `[BIOMETRÍA] Error Engine: ${tfErr.message}`,
      });
    }

    await invoke("log_info", {
      message: `[BIOMETRÍA] Iniciando detección (Res: ${processedInput.width}x${processedInput.height})...`,
    });

    const startTime = performance.now();
    let result;

    // Usamos un Race para detectar si se cuelga
    const detectionPromise = (async () => {
      if (useTiny) {
        // 320 es el estándar de Tiny, debajo de eso pierde mucha precisión
        return await faceapi
          .detectSingleFace(
            processedInput,
            new faceapi.TinyFaceDetectorOptions({ inputSize: 320 }),
          )
          .withFaceLandmarks()
          .withFaceDescriptor();
      } else {
        return await faceapi
          .detectSingleFace(processedInput)
          .withFaceLandmarks()
          .withFaceDescriptor();
      }
    })();

    const timeoutPromise = new Promise((_, reject) =>
      setTimeout(
        () =>
          reject(
            new Error("Detección de rostro excedió el tiempo límite (60s)."),
          ),
        60000,
      ),
    );

    try {
      result = await Promise.race([detectionPromise, timeoutPromise]);
    } catch (timeoutError) {
      await invoke("log_info", {
        message: `[BIOMETRÍA] ERROR: ${timeoutError.message}`,
      });
      return null;
    }

    const duration = (performance.now() - startTime).toFixed(0);

    if (!result) {
      await invoke("log_info", {
        message: `[BIOMETRÍA] No se detectó rostro tras ${duration}ms.`,
      });
      return null;
    }

    await invoke("log_info", {
      message: `[BIOMETRÍA] ¡Análisis exitoso! Tiempo total: ${duration}ms.`,
    });
    return result.descriptor;
  } catch (e) {
    const errorMsg = `[BIOMETRÍA] ERROR: ${e.message}`;
    await invoke("log_info", { message: errorMsg });
    return null;
  }
}

/**
 * Calculate Euclidean distance between two descriptors
 * < 0.6 is usually a match
 */
export function compareFaces(desc1, desc2) {
  return faceapi.euclideanDistance(desc1, desc2);
}

/**
 * Match a face against known descriptors
 */
export function findBestMatch(queryDescriptor, knownDescriptors) {
  const faceMatcher = new faceapi.FaceMatcher(knownDescriptors, 0.6);
  return faceMatcher.findBestMatch(queryDescriptor);
}
