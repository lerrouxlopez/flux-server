import { useState } from "react";
import { Modal } from "../../components/Modal";
import { StartAudio, useRoomContext } from "@livekit/components-react";

export function DeviceSetupDialog(props: { triggerClassName?: string }) {
  const [open, setOpen] = useState(false);
  const room = useRoomContext();

  return (
    <>
      <button
        className={
          props.triggerClassName ??
          "rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800/60"
        }
        onClick={() => setOpen(true)}
        type="button"
      >
        Device setup
      </button>
      <Modal open={open} onClose={() => setOpen(false)} title="Device setup">
        <div className="space-y-3 text-sm text-slate-200">
          <div className="text-slate-300">
            If your browser blocked audio autoplay, enable playback here. (Device selection UI can be expanded later.)
          </div>
          <div>
            <StartAudio room={room} label="Allow audio playback" />
          </div>
        </div>
      </Modal>
    </>
  );
}

