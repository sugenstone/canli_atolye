/**
 * Backend API DTO'larıyla birebir eşleşen tipler (docs/architecture.md §3.4).
 * Backend: application/dto — senkron tutulmalı.
 */

export type Role = 'ADMIN' | 'PROJECT_MANAGER' | 'TEAM_LEADER' | 'WORKER' | 'VIEWER';

export type ProjectStatus =
	| 'DRAFT'
	| 'ACTIVE'
	| 'PAUSED'
	| 'COMPLETED'
	| 'CANCELLED'
	| 'ARCHIVED';

/** Process durumları — Faz 4'te kullanılacak, şimdiden tanımlı. */
export type ProcessStatus =
	| 'PENDING'
	| 'READY'
	| 'IN_PROGRESS'
	| 'PAUSED'
	| 'BLOCKED'
	| 'COMPLETED'
	| 'CANCELLED';

export interface UserDto {
	id: string;
	email: string;
	full_name: string;
	role: Role;
	active: boolean;
	created_at: string;
}

export interface WorkspaceDto {
	id: string;
	name: string;
	slug: string;
	description?: string | null;
}

export interface MeResponse {
	user: UserDto;
	workspace: WorkspaceDto;
}

export interface ProjectDto {
	id: string;
	workspace_id: string;
	name: string;
	code: string;
	description?: string | null;
	status: ProjectStatus;
	planned_start_date?: string | null;
	planned_end_date?: string | null;
	actual_start_date?: string | null;
	actual_end_date?: string | null;
	created_at: string;
	updated_at: string;
}

export interface TeamDto {
	id: string;
	workspace_id: string;
	name: string;
	description?: string | null;
}

// --- Bölümler (recursive sections) ---

export interface SectionDto {
	id: string;
	project_id: string;
	parent_id?: string | null;
	name: string;
	code?: string | null;
	type?: string | null;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface SectionTreeNode extends SectionDto {
	children: SectionTreeNode[];
}

export interface SectionTreeResponse {
	project_id: string;
	total: number;
	nodes: SectionTreeNode[];
}

// --- İş kalemleri (Faz 3) ---

export type Priority = 'LOW' | 'NORMAL' | 'HIGH' | 'URGENT';

export type WorkItemStatus =
	| 'PENDING'
	| 'READY'
	| 'IN_PROGRESS'
	| 'PAUSED'
	| 'BLOCKED'
	| 'COMPLETED'
	| 'CANCELLED';

export interface WorkItemTypeDto {
	id: string;
	workspace_id: string;
	name: string;
	code: string;
	active: boolean;
}

export interface WorkItemDto {
	id: string;
	workspace_id: string;
	project_id: string;
	section_id: string;
	work_item_type_id: string;
	name: string;
	code?: string | null;
	status: WorkItemStatus;
	priority: Priority;
	planned_start_at?: string | null;
	planned_end_at?: string | null;
	created_at: string;
}

export type PropertyDataType =
	| 'TEXT'
	| 'LONG_TEXT'
	| 'NUMBER'
	| 'DECIMAL'
	| 'BOOLEAN'
	| 'DATE'
	| 'DATETIME'
	| 'SELECT'
	| 'MULTI_SELECT';

export interface PropertyDefinitionDto {
	id: string;
	workspace_id: string;
	name: string;
	key: string;
	data_type: PropertyDataType;
	unit?: string | null;
	required: boolean;
	options?: string | null; // JSON array string (SELECT)
}

/** Tanım + mevcut değer birleşik satır (GET /work-items/{id}). */
export interface PropertyValueRow {
	definition_id: string;
	definition_name: string;
	definition_key: string;
	data_type: PropertyDataType;
	unit?: string | null;
	required: boolean;
	options?: string | null;
	value_text?: string | null;
	value_number?: number | null;
	value_boolean?: boolean | null;
}

export interface WorkItemDetailDto {
	item: WorkItemDto;
	properties: PropertyValueRow[];
}

// --- Süreç motoru (Faz 4) ---

export interface ProcessTemplateDto {
	id: string;
	workspace_id: string;
	name: string;
	code: string;
	description?: string | null;
	active: boolean;
}

export interface ProcessGroupDto {
	id: string;
	workspace_id: string;
	name: string;
	description?: string | null;
	active: boolean;
}

export interface ProcessGroupStepDto {
	id: string;
	process_group_id: string;
	process_template_id: string;
	sort_order: number;
	required: boolean;
}

export interface ProcessDependencyDto {
	id: string;
	process_group_step_id: string;
	depends_on_process_group_step_id: string;
	required_status: string;
}

export interface GroupDetailDto {
	group: ProcessGroupDto;
	steps: ProcessGroupStepDto[];
	dependencies: ProcessDependencyDto[];
}

/** Execution + şablon adı + adım sırası (UI listesi). */
export interface ExecutionRow {
	id: string;
	work_item_id: string;
	process_template_id: string;
	process_group_step_id: string;
	status: WorkItemStatus; // aynı değer kümesi (process durumları)
	status_before_block?: string | null;
	version: number;
	assigned_user_id?: string | null;
	assigned_team_id?: string | null;
	ready_at?: string | null;
	started_at?: string | null;
	completed_at?: string | null;
	created_at: string;
	template_name: string;
	sort_order: number;
}

export interface ProcessEventDto {
	id: string;
	process_execution_id: string;
	event_type: string;
	previous_status?: string | null;
	new_status?: string | null;
	user_id: string;
	timestamp: string;
	note?: string | null;
}

// --- Faz 5: Operasyon ---

export interface BlockReasonDto {
	id: string;
	name: string;
	description?: string | null;
	sort_order: number;
}

export interface ProcessBlockDto {
	id: string;
	process_execution_id: string;
	reason_id: string;
	description?: string | null;
	created_by: string;
	created_at: string;
	resolved_by?: string | null;
	resolved_at?: string | null;
	resolution_note?: string | null;
}

export interface AttachmentDto {
	id: string;
	entity_type: string;
	entity_id: string;
	file_name: string;
	storage_key: string;
	mime_type: string;
	size: number;
	uploaded_by: string;
	created_at: string;
}

/** Standart API hata gövdesi (docs/architecture.md §19). */
export interface ApiErrorBody {
	code: string;
	message: string;
	details?: Record<string, unknown>;
}
