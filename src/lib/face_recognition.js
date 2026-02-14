import * as faceapi from 'face-api.js';

// Configuration
const MODEL_URL = '/models';

// State
let modelsLoaded = false;
let labeledFaceDescriptors = []; // To store known faces

/**
 * Load models from public/models
 */
export async function loadModels() {
    if (modelsLoaded) return;
    try {
        console.log("Cargando modelos biométricos...");
        await faceapi.nets.ssdMobilenetv1.loadFromUri(MODEL_URL);
        await faceapi.nets.faceLandmark68Net.loadFromUri(MODEL_URL);
        await faceapi.nets.faceRecognitionNet.loadFromUri(MODEL_URL);
        modelsLoaded = true;
        console.log("Modelos cargados correctamente.");
    } catch (e) {
        console.error("Error cargando modelos face-api:", e);
        throw e;
    }
}

/**
 * Detect face and extract 128-float descriptor
 * @param {HTMLImageElement | HTMLVideoElement | HTMLCanvasElement} input 
 */
export async function getFaceDescriptor(input) {
    if (!modelsLoaded) await loadModels();

    // Detect single face with highest confidence
    const detection = await faceapi.detectSingleFace(input).withFaceLandmarks().withFaceDescriptor();

    if (!detection) {
        return null;
    }
    return detection.descriptor;
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
