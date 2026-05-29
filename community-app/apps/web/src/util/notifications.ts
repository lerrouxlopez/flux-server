import friendlyBlipUrl from "../assets/sounds/friendly_blip.wav";

let cachedAudio: HTMLAudioElement | null = null;

export function playFriendlyBlip() {
  try {
    if (!cachedAudio) {
      cachedAudio = new Audio(friendlyBlipUrl);
      cachedAudio.preload = "auto";
      cachedAudio.volume = 0.6;
    }
    cachedAudio.currentTime = 0;
    void cachedAudio.play().catch(() => {});
  } catch {
    // ignore
  }
}

