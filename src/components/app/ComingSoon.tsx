import { Construction } from "lucide-react";

import { EmptyState } from "@/components/app/EmptyState";

/** Placeholder for tabs that arrive in a later milestone. */
export function ComingSoon({ title }: { title: string }) {
  return (
    <EmptyState
      icon={Construction}
      title={`${title} is coming soon`}
      description="This part of Zesp Connect is planned but not built yet. The Overview and device features are ready to use now."
      className="m-auto"
    />
  );
}
