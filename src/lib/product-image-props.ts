// These props carry an onError handler, which a Server Component can't pass to
// <Image>/<img>: the page fails to render (hampers/[code], Oct 2026).
// client-only turns that mistake into a build error instead.
import "client-only";
import type { SyntheticEvent } from "react";
import { thumbUrl, viewUrl } from "./product-images.ts";

/** <img>/<Image> props showing `src`, falling back to the original if that copy is missing. */
function withFallback(src: string, url: string) {
  return {
    src,
    onError: (e: SyntheticEvent<HTMLImageElement>) => {
      if (e.currentTarget.src !== url) e.currentTarget.src = url;
    },
  };
}
export const thumbProps = (url: string) => withFallback(thumbUrl(url), url);
export const viewProps = (url: string) => withFallback(viewUrl(url), url);
