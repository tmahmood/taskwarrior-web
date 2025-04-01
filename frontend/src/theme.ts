export const SUPPORTED_THEMES = ["taskwarrior-dark", "taskwarrior-light"];
export const THEME_ICONS = ["⚹", "☽", "🌣"];
const STORAGE_THEME_KEY = "TWK_THEME";
const DOM_THEME_KEY = "data-theme";

function getThemeStorage() : string | null {
    const theme = localStorage.getItem(STORAGE_THEME_KEY);
    return theme;
}

function getThemeDom() : string | null {
    const theme = document.getElementsByTagName('html')[0].getAttribute(DOM_THEME_KEY);
    return theme;
}

function getTheme() : string | null {
    const themeStorage = getThemeStorage();
    const themeDom = getThemeDom();

    return themeDom === null ? themeStorage : themeDom;
}

function setTheme(theme: string | null) : boolean {
    if (theme === null) {
        localStorage.removeItem(STORAGE_THEME_KEY);
        document.getElementsByTagName('html')[0].removeAttribute(DOM_THEME_KEY);
    } else {
        localStorage.setItem(STORAGE_THEME_KEY, theme);
        document.getElementsByTagName('html')[0].setAttribute(DOM_THEME_KEY, theme);
    }

    let themeIndex = -1;
    if (theme != null) {
        themeIndex = SUPPORTED_THEMES.indexOf(theme);
    }
    const iconIndex = themeIndex + 1;
    document.getElementById('theme-switcher')?.innerText = THEME_ICONS.at(iconIndex);

    return true;
}

export function switchTheme() {
    const currentTheme = getTheme();
    let themeIndex = -1;
    if (currentTheme != null) {
        themeIndex = SUPPORTED_THEMES.indexOf(currentTheme);
    }
    themeIndex = themeIndex + 1;
    if (themeIndex >= SUPPORTED_THEMES.length) {
        themeIndex = -1;
    }

    if (themeIndex >= 0) {
        setTheme(SUPPORTED_THEMES.at(themeIndex)!);
    } else {
        setTheme(null);
    }
}

export function init() {
    // Ensure, that the icon is set correctly.
    const themeDom = getThemeDom();
    setTheme(themeDom!);
    
    // Check if there is anything different.
    // Local storage is overriding DOM on initial read.
    const theme = getThemeStorage();
    if (theme != null) {
        setTheme(theme!);
    }
}