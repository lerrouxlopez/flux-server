import { create } from "zustand";
import { DEFAULT_THEME_ID } from "../branding/presets";

export const LS_USER_THEME = "flux_user_theme";

type UserThemeState = {
  themeId: string;
  setThemeId: (id: string) => void;
};

export const useUserThemeStore = create<UserThemeState>((set) => ({
  themeId: localStorage.getItem(LS_USER_THEME) ?? DEFAULT_THEME_ID,
  setThemeId: (id) => {
    localStorage.setItem(LS_USER_THEME, id);
    set({ themeId: id });
  },
}));
