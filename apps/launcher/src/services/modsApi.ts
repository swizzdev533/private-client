import { z } from "zod";
import { invokeValidated, listenValidated } from "../lib/tauriBridge";
import {
  commandResultSchema,
  installPlanSchema,
  installedModSchema,
  modSearchRequestSchema,
  modSearchResponseSchema,
  operationProgressSchema,
  pendingOperationSchema,
  type ModSearchRequest,
} from "../types/contracts";

const installedModsSchema = z.array(installedModSchema);
const pendingOperationsSchema = z.array(pendingOperationSchema);

export const modsApi = {
  search: (request: ModSearchRequest) =>
    invokeValidated("search_modrinth", modSearchResponseSchema, {
      request: modSearchRequestSchema.parse(request),
    }),
  installPlan: (projectId: string) =>
    invokeValidated("get_mod_install_plan", installPlanSchema, { projectId }),
  install: (projectId: string, versionId: string) =>
    invokeValidated("install_mod", commandResultSchema, {
      projectId,
      versionId,
    }),
  installed: () => invokeValidated("list_installed_mods", installedModsSchema),
  pending: () => invokeValidated("list_pending_operations", pendingOperationsSchema),
  remove: (projectId: string) =>
    invokeValidated("remove_mod", commandResultSchema, { projectId }),
  update: (projectId: string) =>
    invokeValidated("update_mod", commandResultSchema, { projectId }),
  cancelPending: (operationId: string) =>
    invokeValidated("cancel_pending_operation", commandResultSchema, {
      operationId,
    }),
  applyPending: () => invokeValidated("apply_pending_operations", commandResultSchema),
  downloadOptifine: () => invokeValidated("download_optifine", commandResultSchema),
  importOptifine: () => invokeValidated("import_optifine", commandResultSchema),
};

export function subscribeOperationProgress(
  handler: (payload: z.infer<typeof operationProgressSchema>) => void,
): Promise<() => void> {
  return listenValidated("launcher://operation-progress", operationProgressSchema, handler);
}
