import { describe, expect, it } from "vitest";
import { defaultIntent } from "./useMediaJoin";

describe("defaultIntent", () => {
  it("uses mediaDefaults.join_intent for meeting rooms", () => {
    expect(
      defaultIntent({
        roomKind: "meeting",
        mediaDefaults: {
          room_kind_preference: "meeting",
          join_intent: "video",
          auto_publish_audio: true,
          auto_publish_video: true,
          auto_publish_screen: false,
          auto_subscribe: true,
        },
      }),
    ).toBe("video");
  });

  it("forces voice_only for voice rooms", () => {
    expect(
      defaultIntent({
        roomKind: "voice",
        mediaDefaults: {
          room_kind_preference: "meeting",
          join_intent: "video",
          auto_publish_audio: true,
          auto_publish_video: true,
          auto_publish_screen: false,
          auto_subscribe: true,
        },
      }),
    ).toBe("voice_only");
  });

  it("maps stage intent based on room_kind_preference", () => {
    expect(
      defaultIntent({
        roomKind: "stage",
        mediaDefaults: {
          room_kind_preference: "voice",
          join_intent: "voice_only",
          auto_publish_audio: true,
          auto_publish_video: false,
          auto_publish_screen: false,
          auto_subscribe: true,
        },
      }),
    ).toBe("stage_viewer");
    expect(
      defaultIntent({
        roomKind: "stage",
        mediaDefaults: {
          room_kind_preference: "meeting",
          join_intent: "video",
          auto_publish_audio: true,
          auto_publish_video: true,
          auto_publish_screen: false,
          auto_subscribe: true,
        },
      }),
    ).toBe("stage_speaker");
  });
});

