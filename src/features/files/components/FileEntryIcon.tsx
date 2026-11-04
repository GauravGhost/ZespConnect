import {
  File,
  FileArchive,
  FileCode,
  FileImage,
  FileMusic,
  FileSpreadsheet,
  FileText,
  FileVideo,
  Folder,
  type LucideIcon,
  Package,
} from "lucide-react";

import { cn } from "@/lib/utils";
import type { FileEntry } from "@/types";
import { type FileKind, fileKind } from "../fileKind";

const KIND_ICONS: Record<FileKind, { icon: LucideIcon; tone: string }> = {
  image: { icon: FileImage, tone: "text-sky-500" },
  video: { icon: FileVideo, tone: "text-violet-500" },
  audio: { icon: FileMusic, tone: "text-pink-500" },
  document: { icon: FileText, tone: "text-amber-500" },
  spreadsheet: { icon: FileSpreadsheet, tone: "text-emerald-500" },
  archive: { icon: FileArchive, tone: "text-orange-500" },
  app: { icon: Package, tone: "text-lime-600" },
  code: { icon: FileCode, tone: "text-teal-500" },
  other: { icon: File, tone: "text-muted-foreground" },
};

export function FileEntryIcon({ entry }: { entry: FileEntry }) {
  if (entry.is_directory) return <Folder className="text-primary size-4 shrink-0" />;
  const { icon: Icon, tone } = KIND_ICONS[fileKind(entry.name)];
  return <Icon className={cn("size-4 shrink-0", tone)} />;
}
