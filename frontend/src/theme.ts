/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

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

function setTheme(theme: string | null, overrideStorage: boolean = true) : boolean {
    if (theme === null) {
        if (overrideStorage) {
            localStorage.removeItem(STORAGE_THEME_KEY);
        }
        document.getElementsByTagName('html')[0].removeAttribute(DOM_THEME_KEY);
    } else {
        if (overrideStorage) {
            localStorage.setItem(STORAGE_THEME_KEY, theme);
        }
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
    // If a theme is already set on storage, force it!
    const theme = getThemeStorage();
    if (theme != null) {
        setTheme(theme!);
        return;
    }

    // Ensure, that the icon is set correctly.
    // Do not override the storage!
    // This part is only done, if nothing is given yet in storage.
    const themeDom = getThemeDom();
    setTheme(themeDom!, false);
}