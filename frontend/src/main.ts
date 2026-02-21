/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

import 'htmx.org';
import 'hyperscript.org';
import * as _hyperscript from "hyperscript.org";
import hotkeys from "hotkeys-js";
import * as theme from "./theme";

_hyperscript.browserInit();

export const doing_something = () => {
    console.log("Hello world");
}

hotkeys.filter = function (event) {
    // @ts-ignore
    let tagName = event.target.tagName;
    hotkeys.setScope(/^(INPUT|TEXTAREA|SELECT)$/.test(tagName) ? 'input' : 'other');
    return true;
};

function focusTextInput(event: KeyboardEvent | MouseEvent) {
    let ss = document.getElementById('task-details-inp');
    if (ss !== null) {
        event.preventDefault()
        ss.focus();
    } else {
        document.getElementById('cmd-inp')?.focus();
    }
}

window.handleTaskAnnotations = (event: KeyboardEvent | MouseEvent) => {
    if(event.target != document.getElementById('btn-denotate-task')) {
        console.log("not processing")
        return
    }
    event.preventDefault();
    let annoSelector = document.getElementById('anno-inp');
    document.querySelector('#anno-inp')?.classList.toggle('hidden');
    Array.from(document.querySelectorAll('.is-a-annotation')).forEach((value) => {
        value.classList.toggle('hidden');
    });
    if (annoSelector?.checkVisibility()) {
        annoSelector.focus();
    }
    return false;
};

window.handleTaskAnnotationTrigger = (event: KeyboardEvent | MouseEvent) => {
    event.preventDefault();
    if (event.target) {
        let shortkey = event.target.value;
        if (shortkey.length >= 2) { 
            let element = document.getElementById("anno_dlt_" + shortkey);
            if (element) {
                element.click();
            }
        };
    }
}

hotkeys('esc', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    if(event.target != document.getElementById('tag-inp') &&
        event.target != document.getElementById('query-inp')) {
        console.log("not processing")
        return
    }
    event.preventDefault();
    let tag_selector = document.getElementById('cmd-inp');
    if (event.target == document.getElementById('tag-inp')) {
        document.querySelector('#tags_map_drawer')?.classList.toggle('hidden');
    } else if (event.target == document.getElementById('query-inp')) {
        document.querySelector('#querys_map_drawer')?.classList.toggle('hidden');
    }
    if (tag_selector?.checkVisibility()) {
        tag_selector.focus();
    }
    return false;
});

hotkeys('ctrl+shift+K', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    event.preventDefault();
    focusTextInput(event);
    return false;
});

hotkeys('t', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    if(event.target != document.getElementById('cmd-inp')) {
        console.debug("not processing")
        return
    }
    event.preventDefault()
    window['togglePanel']('tag');
});

hotkeys('q', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    if(event.target != document.getElementById('cmd-inp')) {
        console.debug("not processing")
        return
    }
    event.preventDefault()
    window['togglePanel']('query');
});

window['togglePanel'] = (panelType: string) => {
    let tagSelector = document.getElementById(panelType + '-inp')
    document.querySelector('#' + panelType + 's_map_drawer')?.classList.toggle('hidden')
    if (tagSelector?.checkVisibility()) {
        tagSelector.focus();
    }
    return false;
};

window['processPanelShortcut'] = (event: KeyboardEvent, panelType: string) {
    const shortcut = event.target?.value;
    if (shortcut.length >= 2) { 
        document.getElementById(panelType + "s_" + shortcut)?.click() 
    };
};

document.addEventListener('click', function (event) {
    let element = document.getElementsByTagName('html')[0];
    switch(event.target) {
        case element:
            focusTextInput(event);
            break;
        case document.getElementById('theme-switcher'):
            event.preventDefault();
            theme.switchTheme();
            break;
    }
    return;
})

document.addEventListener("DOMContentLoaded", function () {
    theme.init();

    let n = setInterval(
        () => {
            let whichOne = 0;
            // document.getElementById('active-timer').querySelectorAll('span.timer-duration')[0]
            let dd = document.getElementById('active-timer');
            if (dd === undefined || dd === null) {
                return
            }
            let timeBox = dd.children[1].children[whichOne];
            let s = timeBox.textContent.split(":");
            let second = parseInt(s.pop());
            let minute = parseInt(s.pop());
            if (isNaN(minute)) {
                minute = 0;
            }
            let hour = parseInt(s.pop());
            if (isNaN(hour)) {
                hour = 0;
            }
            second += 1;
            if (second >= 60) {
                second = 0;
                minute += 1;
                if (minute > 60) {
                    hour += 1;
                }
            }
            timeBox.textContent = hour.toString()
                    // @ts-ignore
                    .padStart(2, "0") + ":" +
                minute.toString()
                    // @ts-ignore
                    .padStart(2, "0") + ":" +
                second.toString()
                    // @ts-ignore
                    .padStart(2, "0");
        }, 1000
    )

    let dayProgress = setInterval(
        () => {
            const dd = document.getElementById('time_of_the_day');
            if (dd === undefined || dd === null) {
                return
            }
            // <progress id="time_of_the_day" class="fill-amber-200 bg-blue-900 w-full shadow-inner shadow-blue-950" max="100" value=""></progress>
            const now = new Date();
            const totalMinutesPassed = now.getMinutes() + (now.getHours() * 60);
            const totalMinutesInDay = 24 * 60;
            const hoursLeft = 24 - now.getHours();
            dd.style.width = totalMinutesPassed * 100 / totalMinutesInDay + "%";
            dd.children[0].children[0].innerHTML = hoursLeft + "h";
        }, 1000
    )
});

