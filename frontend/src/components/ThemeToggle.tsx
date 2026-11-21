import { useState } from "react";
import { useTranslation } from "react-i18next";
import { getStoredTheme, setTheme, type ThemeChoice } from "../lib/theme";

const CHOICES: ThemeChoice[] = ["system", "light", "dark"];

export function ThemeToggle() {
  const { t } = useTranslation();
  const [choice, setChoice] = useState<ThemeChoice>(getStoredTheme);

  return (
    <div className="theme-toggle" role="group" aria-label={t("theme.label")}>
      {CHOICES.map((value) => (
        <button
          key={value}
          type="button"
          aria-pressed={choice === value}
          className={choice === value ? "theme-on" : ""}
          onClick={() => {
            setTheme(value);
            setChoice(value);
          }}
        >
          {t(`theme.${value}`)}
        </button>
      ))}
    </div>
  );
}
