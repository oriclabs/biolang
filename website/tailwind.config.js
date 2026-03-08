/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './*.html',
    './docs/**/*.html',
    './components/**/*.html',
  ],
  darkMode: 'class',
  theme: {
    extend: {
      maxWidth: {
        '8xl': '90rem',
      },
    },
  },
  plugins: [],
}
