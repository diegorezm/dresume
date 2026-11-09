import { isTauri } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus = "idle" | "checking" | "upToDate" | "available" | "downloading" | "error";

/** App updates from GitHub releases (signed `latest.json`). */
class Updater {
  status = $state<UpdateStatus>("idle");
  current = $state("");
  version = $state("");
  progress = $state(0);
  dismissed = $state(false);
  #update: Update | null = null;

  get supported() {
    return isTauri() && !import.meta.env.DEV;
  }

  async check() {
    if (!this.supported || this.status === "checking" || this.status === "downloading") return;
    this.current ||= await getVersion();
    this.status = "checking";
    try {
      this.#update = await check();
      this.version = this.#update?.version ?? "";
      this.status = this.#update ? "available" : "upToDate";
    } catch (e) {
      console.error("update check failed", e);
      this.status = "error";
    }
  }

  async install() {
    if (!this.#update) return;
    this.status = "downloading";
    this.progress = 0;
    let total = 0;
    let done = 0;
    try {
      await this.#update.downloadAndInstall((ev) => {
        if (ev.event === "Started") total = ev.data.contentLength ?? 0;
        else if (ev.event === "Progress") {
          done += ev.data.chunkLength;
          this.progress = total ? done / total : 0;
        }
      });
      await relaunch();
    } catch (e) {
      console.error("update failed", e);
      this.status = "error";
    }
  }
}

export const updater = new Updater();
