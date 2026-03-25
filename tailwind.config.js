/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        canvas: "#05070d",
      },
      boxShadow: {
        glow: "0 0 0 1px rgba(103, 232, 249, 0.25), 0 10px 30px rgba(0, 0, 0, 0.35)",
      },
      backgroundImage: {
        "aurora-core":
          "radial-gradient(circle at 10% 20%, rgba(14,165,233,0.28) 0%, transparent 34%), radial-gradient(circle at 85% 15%, rgba(99,102,241,0.22) 0%, transparent 38%), radial-gradient(circle at 55% 90%, rgba(34,197,94,0.16) 0%, transparent 32%)",
      },
    },
  },
  plugins: [],
};
