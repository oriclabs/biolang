// Theme initializer — must run before page renders to prevent flash
if (localStorage.getItem("theme") === "light") {
  document.documentElement.classList.remove("dark");
}
