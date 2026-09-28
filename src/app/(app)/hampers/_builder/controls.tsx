"use client";

import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, X } from "lucide-react";

/** A labelled read-only figure in the pricing strip. */
export function Cell({
  label,
  value,
  tone,
  strong,
}: {
  label: string;
  value: string;
  tone?: "good" | "bad";
  strong?: boolean;
}) {
  return (
    <div>
      <div className="label">{label}</div>
      <div
        className={[
          "mt-1 tabular-nums",
          strong ? "text-base font-semibold" : "text-sm font-medium",
          tone === "bad" ? "text-red-700" : tone === "good" ? "text-green-800" : "",
        ].join(" ")}
      >
        {value}
      </div>
    </div>
  );
}

export function NumField({
  id,
  label,
  value,
  onChange,
  disabled,
  hint,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
  hint?: string;
}) {
  return (
    <div className="w-30">
      <label className="label" htmlFor={id} title={hint}>
        {label}
      </label>
      <input
        id={id}
        inputMode="decimal"
        className="input input-num mt-1"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
      />
    </div>
  );
}

/**
 * Searchable combobox dropdown picker.
 *
 * - Auto-selects text on focus so typing replaces old value without manual deleting
 * - Shows all options when opening with an existing value (no need to backspace to browse)
 * - One-click clear button (×) to empty field and view all options instantly
 * - Renders via portal with fixed positioning so table overflow-x never clips the menu
 * - Smooth keyboard navigation (ArrowUp, ArrowDown, Enter, Escape)
 */
export function Combo({
  label,
  value,
  options,
  allOptions,
  placeholder,
  disabled,
  onPick,
}: {
  label: string;
  value: string;
  options: string[];
  allOptions?: string[];
  placeholder?: string;
  disabled?: boolean;
  onPick: (text: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const [mounted, setMounted] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number; width: number; transform: string }>({
    top: 0,
    left: 0,
    width: 0,
    transform: "none",
  });

  useEffect(() => {
    setMounted(true);
  }, []);

  // Synchronize search with value whenever value prop changes and user is not actively typing
  useEffect(() => {
    if (!isTyping) {
      setSearch(value);
    }
  }, [value, isTyping]);

  // When actively typing: search across allOptions (or options)
  // When not typing: show options (e.g. current category options)
  const filteredOptions = useMemo(() => {
    if (!isTyping || !search.trim()) {
      return options;
    }
    const q = search.trim().toLowerCase();
    const source = allOptions && allOptions.length > 0 ? allOptions : options;

    const inCurrent: string[] = [];
    const inOther: string[] = [];
    const currentSet = new Set(options);

    for (const opt of source) {
      if (opt.toLowerCase().includes(q)) {
        if (currentSet.has(opt)) {
          inCurrent.push(opt);
        } else {
          inOther.push(opt);
        }
      }
    }

    return [...inCurrent, ...inOther];
  }, [options, allOptions, isTyping, search]);

  const updatePosition = useCallback(() => {
    if (!inputRef.current) return;
    const rect = inputRef.current.getBoundingClientRect();
    const dropdownMaxHeight = 250;
    const spaceBelow = window.innerHeight - rect.bottom;
    const showAbove = spaceBelow < dropdownMaxHeight && rect.top > dropdownMaxHeight;
    const width = Math.max(rect.width, 280);
    const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
    const top = showAbove ? rect.top - 4 : rect.bottom + 4;

    setCoords({
      top,
      left,
      width,
      transform: showAbove ? "translateY(-100%)" : "none",
    });
  }, []);

  // Set highlighted index when dropdown opens or filtered options change
  useEffect(() => {
    if (!isOpen) {
      setHighlightedIndex(-1);
      return;
    }
    if (!isTyping && value) {
      const idx = filteredOptions.indexOf(value);
      setHighlightedIndex(idx >= 0 ? idx : 0);
    } else {
      setHighlightedIndex(filteredOptions.length > 0 ? 0 : -1);
    }
  }, [isOpen, isTyping, value, filteredOptions]);

  // Scroll active item into view
  useEffect(() => {
    if (isOpen && highlightedIndex >= 0 && dropdownRef.current) {
      const el = dropdownRef.current.querySelector(`[data-index="${highlightedIndex}"]`) as HTMLElement | null;
      el?.scrollIntoView({ block: "nearest" });
    }
  }, [isOpen, highlightedIndex]);

  // Handle position tracking, scroll and outside click
  useEffect(() => {
    if (!isOpen) return;

    updatePosition();

    const handleScroll = (e: Event) => {
      if (dropdownRef.current?.contains(e.target as Node)) return;
      updatePosition();
    };

    window.addEventListener("scroll", handleScroll, true);
    window.addEventListener("resize", updatePosition);

    const handleClickOutside = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node) &&
        dropdownRef.current &&
        !dropdownRef.current.contains(e.target as Node)
      ) {
        setIsOpen(false);
        setIsTyping(false);
        setSearch(value);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);

    return () => {
      window.removeEventListener("scroll", handleScroll, true);
      window.removeEventListener("resize", updatePosition);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen, updatePosition, value]);

  const handleSelect = (option: string) => {
    const selected = option === "All categories" ? "" : option;
    setSearch(selected);
    setIsTyping(false);
    setIsOpen(false);
    onPick(selected);
  };

  const handleClear = (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    setSearch("");
    setIsTyping(false);
    onPick("");
    setIsOpen(true);
    inputRef.current?.focus();
  };

  const handleToggle = (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    if (disabled) return;
    if (isOpen) {
      setIsOpen(false);
      setIsTyping(false);
      setSearch(value);
    } else {
      setIsOpen(true);
      setIsTyping(false);
      setSearch(value);
      inputRef.current?.focus();
      inputRef.current?.select();
    }
  };

  const handleFocus = (e: React.FocusEvent<HTMLInputElement>) => {
    e.target.select();
    setIsTyping(false);
    setSearch(value);
    setIsOpen(true);
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setIsTyping(true);
    setSearch(val);
    setIsOpen(true);
    if (options.includes(val)) {
      onPick(val);
    } else if (val === "") {
      onPick("");
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (!isOpen) {
        setIsOpen(true);
        setIsTyping(false);
      } else if (filteredOptions.length > 0) {
        setHighlightedIndex((prev) => (prev + 1) % filteredOptions.length);
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (!isOpen) {
        setIsOpen(true);
        setIsTyping(false);
      } else if (filteredOptions.length > 0) {
        setHighlightedIndex((prev) => (prev <= 0 ? filteredOptions.length - 1 : prev - 1));
      }
    } else if (e.key === "Enter") {
      if (isOpen && highlightedIndex >= 0 && highlightedIndex < filteredOptions.length) {
        e.preventDefault();
        handleSelect(filteredOptions[highlightedIndex]);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      setIsOpen(false);
      setIsTyping(false);
      setSearch(value);
    } else if (e.key === "Tab") {
      if (isOpen) {
        if (isTyping && highlightedIndex >= 0 && highlightedIndex < filteredOptions.length) {
          handleSelect(filteredOptions[highlightedIndex]);
        } else {
          setIsOpen(false);
          setIsTyping(false);
          setSearch(value);
        }
      }
    }
  };

  const renderOptionContent = (opt: string) => {
    const dashIdx = opt.lastIndexOf(" — ");
    if (dashIdx > 0) {
      const name = opt.slice(0, dashIdx);
      const code = opt.slice(dashIdx + 3);
      return (
        <div className="flex items-center justify-between w-full min-w-0 pr-1">
          <span className="truncate">{name}</span>
          <span className="ml-2 shrink-0 font-mono text-[11px] text-(--color-muted) font-normal">
            {code}
          </span>
        </div>
      );
    }
    return <span className="truncate pr-1">{opt}</span>;
  };

  const displayValue = isOpen && isTyping ? search : value;

  return (
    <div ref={containerRef} className="relative w-full min-w-0">
      <input
        ref={inputRef}
        aria-label={label}
        className={`input pr-12 truncate text-xs sm:text-sm ${
          disabled ? "cursor-not-allowed" : "cursor-pointer focus:cursor-text"
        }`}
        placeholder={placeholder}
        value={displayValue}
        disabled={disabled}
        onFocus={handleFocus}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
      />
      <div className="absolute right-1 top-1/2 -translate-y-1/2 flex items-center gap-0.5">
        {(value || (isOpen && search)) && !disabled && (
          <button
            type="button"
            title={`Clear ${label}`}
            aria-label={`Clear ${label}`}
            onClick={handleClear}
            className="p-1 rounded text-(--color-muted) hover:text-red-700 hover:bg-red-50 transition-colors"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        )}
        <button
          type="button"
          tabIndex={-1}
          title={`Show ${label} options`}
          aria-label={`Show ${label} options`}
          disabled={disabled}
          onClick={handleToggle}
          className="p-1 rounded text-(--color-muted) hover:text-(--color-ink) hover:bg-slate-100 transition-colors"
        >
          <ChevronDown
            className={`h-3.5 w-3.5 transition-transform duration-150 ${isOpen ? "rotate-180" : ""}`}
          />
        </button>
      </div>

      {mounted &&
        isOpen &&
        createPortal(
          <div
            ref={dropdownRef}
            style={{
              position: "fixed",
              top: coords.top,
              left: coords.left,
              width: coords.width,
              transform: coords.transform,
              zIndex: 9999,
            }}
            className="max-h-60 overflow-y-auto rounded-lg border border-(--color-line) bg-white py-1 shadow-xl text-xs sm:text-sm"
          >
            {filteredOptions.length === 0 ? (
              <div className="px-3 py-3 text-center text-xs text-(--color-muted)">
                No matching options
              </div>
            ) : (
              filteredOptions.map((opt, idx) => {
                const isSelected = opt === value || (value === "" && opt === "All categories");
                const isHighlighted = idx === highlightedIndex;
                return (
                  <button
                    key={opt}
                    type="button"
                    data-index={idx}
                    onClick={() => handleSelect(opt)}
                    onMouseEnter={() => setHighlightedIndex(idx)}
                    className={`flex w-full select-none items-center justify-between px-3 py-1.5 text-left cursor-pointer transition-colors ${
                      isHighlighted
                        ? "bg-paper text-(--color-ink)"
                        : isSelected
                        ? "bg-slate-50 text-(--color-ink) font-semibold"
                        : "text-(--color-ink)"
                    }`}
                  >
                    {renderOptionContent(opt)}
                    {isSelected && (
                      <Check className="h-3.5 w-3.5 shrink-0 text-(--color-brand) ml-1" />
                    )}
                  </button>
                );
              })
            )}
          </div>,
          document.body
        )}
    </div>
  );
}

export function RowButton({
  label,
  onClick,
  disabled,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
      className={`rounded px-1.5 py-0.5 text-(--color-muted) disabled:opacity-30 ${
        danger ? "hover:bg-red-50 hover:text-red-700" : "hover:bg-paper"
      }`}
    >
      {children}
    </button>
  );
}
