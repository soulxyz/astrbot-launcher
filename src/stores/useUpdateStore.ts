import { create } from 'zustand';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

interface UpdateState {
  hasUpdate: boolean;
  newVersion: string;
  releaseNotes: string;
  checking: boolean;
  installing: boolean;
  checkForUpdate: () => Promise<void>;
  installUpdate: () => Promise<void>;
}

let cachedUpdate: Update | null = null;

export const useUpdateStore = create<UpdateState>((set, get) => ({
  hasUpdate: false,
  newVersion: '',
  releaseNotes: '',
  checking: false,
  installing: false,

  checkForUpdate: async () => {
    if (get().checking) return;
    set({ checking: true });
    try {
      const update = await check();
      if (update) {
        cachedUpdate = update;
        set({
          hasUpdate: true,
          newVersion: update.version,
          releaseNotes: update.body ?? '',
        });
      } else {
        cachedUpdate = null;
        set({ hasUpdate: false, newVersion: '', releaseNotes: '' });
      }
    } catch (e) {
      cachedUpdate = null;
      set({ hasUpdate: false, newVersion: '', releaseNotes: '' });
      console.error('Update check failed:', e);
      throw e;
    } finally {
      set({ checking: false });
    }
  },

  installUpdate: async () => {
    if (!cachedUpdate) return;
    set({ installing: true });
    try {
      await cachedUpdate.downloadAndInstall();
      await relaunch();
    } catch (e) {
      console.error('Update install failed:', e);
      throw e;
    } finally {
      set({ installing: false });
    }
  },
}));
