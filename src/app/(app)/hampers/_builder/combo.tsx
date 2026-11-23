"use client";

import { useMemo, useRef, useState } from "react";
import { ChevronDown, X } from "lucide-react";
import { filterComboOptions } from "./combo-options";
import { ComboMenu } from "./combo-menu";
import { useFloatingMenu } from "./use-floating-menu";

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

  // What the box searches: the typed text while typing, otherwise the picked value.
  const query = isTyping ? search : value;
  const filteredOptions = useMemo(
    () => filterComboOptions(options, allOptions, query, isTyping),
    [options, allOptions, isTyping, query],
  );

  // Close and put the picked value back.
  const close = () => {
    setIsOpen(false);
    setIsTyping(false);
    setSearch(value);
  };
  const coords = useFloatingMenu(isOpen, { input: inputRef, container: containerRef, menu: dropdownRef }, close);

  // Re-pick the highlighted row when the menu opens or its options change
  // (during render, not in an effect, so there's no extra paint).
  const highlightDeps = [isOpen, isTyping, value, filteredOptions] as const;
  const [seenDeps, setSeenDeps] = useState(highlightDeps);
  if (highlightDeps.some((d, i) => d !== seenDeps[i])) {
    setSeenDeps(highlightDeps);
    const idx = !isOpen ? -1 : !isTyping && value ? filteredOptions.indexOf(value) : -1;
    setHighlightedIndex(!isOpen ? -1 : idx >= 0 ? idx : filteredOptions.length > 0 ? 0 : -1);
  }

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
    if (isOpen) return close();
    setIsOpen(true);
    setIsTyping(false);
    setSearch(value);
    inputRef.current?.focus();
    inputRef.current?.select();
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
    if (options.includes(val)) onPick(val);
    else if (val === "") onPick("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    const n = filteredOptions.length;
    const highlighted = highlightedIndex >= 0 && highlightedIndex < n;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!isOpen) {
        setIsOpen(true);
        setIsTyping(false);
      } else if (n > 0) {
        setHighlightedIndex((prev) => (e.key === "ArrowDown" ? (prev + 1) % n : prev <= 0 ? n - 1 : prev - 1));
      }
    } else if (e.key === "Enter") {
      if (isOpen && highlighted) {
        e.preventDefault();
        handleSelect(filteredOptions[highlightedIndex]);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "Tab" && isOpen) {
      if (isTyping && highlighted) handleSelect(filteredOptions[highlightedIndex]);
      else close();
    }
  };

  const displayValue = isOpen && isTyping ? query : value;

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
        {(value || (isOpen && query)) && !disabled && (
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
          <ChevronDown className={`h-3.5 w-3.5 transition-transform duration-150 ${isOpen ? "rotate-180" : ""}`} />
        </button>
      </div>

      {isOpen && (
        <ComboMenu
          menuRef={dropdownRef}
          coords={coords}
          options={filteredOptions}
          value={value}
          highlightedIndex={highlightedIndex}
          onSelect={handleSelect}
          onHighlight={setHighlightedIndex}
        />
      )}
    </div>
  );
}
