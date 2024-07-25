import 'htmx.org';
import 'hyperscript.org';
import * as _hyperscript from "hyperscript.org";
import hotkeys from "hotkeys-js";

_hyperscript.browserInit();

export const doing_something = () => {
    console.log("Hello world");
}

hotkeys.filter = function (event) {
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


document.addEventListener("DOMContentLoaded", function () {
    let n = setInterval(
        () => {
            let which_one = 1;
            let dd = document.getElementById('active-timer');
            if (dd === undefined || dd === null) {
                return
            }
            let s = dd.children[which_one].textContent.split(":");
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
            let final_time =
                hour.toString().padStart(2, "0") + ":" +
                minute.toString().padStart(2, "0") + ":" +
                second.toString().padStart(2, "0")
            dd.children[which_one].textContent = final_time;
        }, 1000
    )
});