import * as Lucide from "lucide-react";
import type { LucideIcon, LucideProps } from "lucide-react";
function icon(Component: LucideIcon) {
  return function Icon({ weight, ...props }: LucideProps & { weight?: string }) {
    return (
      <Component strokeWidth={1.7} fill={weight === "fill" ? "currentColor" : "none"} {...props} />
    );
  };
}
export const ArrowClockwise = icon(Lucide.RefreshCw);
export const ArrowCounterClockwise = icon(Lucide.RotateCcw);
export const ArrowRight = icon(Lucide.ArrowRight);
export const ArrowSquareOut = icon(Lucide.ExternalLink);
export const Broadcast = icon(Lucide.Radio);
export const CaretDown = icon(Lucide.ChevronDown);
export const Check = icon(Lucide.Check);
export const CloudSlash = icon(Lucide.CloudOff);
export const Cpu = icon(Lucide.Cpu);
export const DiscordLogo = icon(Lucide.Headset);
export const DownloadSimple = icon(Lucide.Download);
export const FileArrowUp = icon(Lucide.FileUp);
export const FloppyDisk = icon(Lucide.Save);
export const Headphones = icon(Lucide.Headphones);
export const Heart = icon(Lucide.Heart);
export const Info = icon(Lucide.Info);
export const MagnifyingGlass = icon(Lucide.Search);
export const MicrophoneSlash = icon(Lucide.MicOff);
export const Pause = icon(Lucide.Pause);
export const Play = icon(Lucide.Play);
export const Plus = icon(Lucide.Plus);
export const Record = icon(Lucide.Circle);
export const ShieldCheck = icon(Lucide.ShieldCheck);
export const SlidersHorizontal = icon(Lucide.SlidersHorizontal);
export const Sparkle = icon(Lucide.Sparkles);
export const SpeakerHigh = icon(Lucide.Volume2);
export const SpeakerSlash = icon(Lucide.VolumeX);
export const Square = icon(Lucide.Square);
export const Star = icon(Lucide.Star);
export const Stop = icon(Lucide.Square);
export const Trash = icon(Lucide.Trash2);
export const UploadSimple = icon(Lucide.Upload);
export const Warning = icon(Lucide.TriangleAlert);
export const WarningCircle = icon(Lucide.CircleAlert);
export const X = icon(Lucide.X);
