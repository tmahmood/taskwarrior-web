import 'htmx.org';
import 'hyperscript.org';
import * as _hyperscript from "hyperscript.org";
import hotkeys from "hotkeys-js";

_hyperscript.browserInit();

export const doing_something = () => {
    console.log("Hello world");
}

hotkeys.filter = function(event) {
    // @ts-ignore
    let tagName = event.target.tagName;
    hotkeys.setScope(/^(INPUT|TEXTAREA|SELECT)$/.test(tagName) ? 'input' : 'other');
    return true;
}


hotkeys('ctrl+shift+K', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    event.preventDefault()
    let ss = document.getElementById('task-details-inp');
    console.log(ss);
    if (ss !== null) {
        event.preventDefault()
        ss.focus();
    } else {
        document.getElementById('cmd-inp').focus();
    }
    return false;
});

hotkeys('ctrl+shift+L', function (event, handler) {
    // Prevent the default refresh event under WINDOWS system
    event.preventDefault()
    document.getElementById('tag-inp').focus();
});

// hotkeys('t', function (event, handler) {
//     // Prevent the default refresh event under WINDOWS system
//     event.preventDefault()
//     let foundTags = [];
//     document.querySelectorAll('.tg-col button').forEach(value => {
//         let tg = value.textContent.trim();
//         for (const foundTagsKey of foundTags) {
//             console.log(`${tg} - ${foundTagsKey}`);
//             if (tg === foundTagsKey) {
//                 return;
//             }
//         }
//         foundTags.push(tg)
//         console.log(value.classList);
//         value.classList.add('red');
//         console.log();
//     });
// });


document.addEventListener("DOMContentLoaded", function() {


});