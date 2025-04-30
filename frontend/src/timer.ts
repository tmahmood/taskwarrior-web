var global_twk_timer: number|null = null;

function updateTimer() {
    let timer_dom = document.getElementById('active-timer');
    if (timer_dom === null) {
        return;
    }

    let timer_box_dom = timer_dom.querySelector(".timer-duration");
    if (timer_box_dom === null) {
        return;
    }

    let task_start = timer_dom?.getAttribute("data-task-start");
    if (task_start === null || task_start === undefined) {
        return;
    }
    let task_start_dt = Date.parse(task_start);
    let now_utc = Date.now();
    let diff_s = Math.floor((now_utc - task_start_dt)/1000);
    let hours = Math.floor(diff_s/3600);
    let minutes = Math.floor((diff_s - (hours*3600))/60);
    let seconds = Math.floor((diff_s - (hours*3600)) - (minutes*60));
    timer_box_dom.textContent = 
        hours.toString().padStart(2, "0") + ":" + 
        minutes.toString().padStart(2, "0") + ":" + 
        seconds.toString().padStart(2, "0")   
}

export function init() {
    global_twk_timer = setInterval(
        updateTimer,
        1000
    )
}

export function stop() {
    if (global_twk_timer != null && global_twk_timer != undefined) {
        clearInterval(global_twk_timer);
        global_twk_timer = null;
    }
}