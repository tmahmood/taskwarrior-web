import 'htmx.org';
import 'hyperscript.org';
import * as _hyperscript from "hyperscript.org";
import hotkeys from "hotkeys-js";

_hyperscript.browserInit();
export const doing_something = () => {
    console.log("Hello world");
}


hotkeys('ctrl+shift+K', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    event.preventDefault()
    document.getElementById('cmd-inp').focus();
});

hotkeys('t', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    event.preventDefault()
});
