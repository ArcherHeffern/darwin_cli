const left_sidebar = document.getElementById("left-sidebar");
const right_sidebar = document.getElementById("right-sidebar");
const bottom_sidebar = document.getElementById("bottom-sidebar");

// Clicking on each of the buttons will set "open" class on the corresponding sidebar. 
// Use CSS to style

const left_sidebar_button = document.getElementById("left-sidebar-button");
const right_sidebar_button = document.getElementById("right-sidebar-button");
const bottom_sidebar_button = document.getElementById("bottom-sidebar-button");

if (left_sidebar && left_sidebar_button) {
    left_sidebar_button.addEventListener("click", (e) => {
        left_sidebar.classList.toggle("open");
    });
}

if (right_sidebar && right_sidebar_button) {
    right_sidebar_button.addEventListener("click", (e) => {
        right_sidebar.classList.toggle("open");
    });
}

if (bottom_sidebar && bottom_sidebar_button) {
    bottom_sidebar_button.addEventListener("click", (e) => {
        bottom_sidebar.classList.toggle("open");
    });
}