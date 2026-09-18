/** Work item servisi — tipler, iş kalemleri, dinamik özellikler */

import { api } from './client';
import type {
	Priority,
	PropertyDefinitionDto,
	WorkItemDetailDto,
	WorkItemDto,
	WorkItemTypeDto
} from '$lib/types';

export interface CreateWorkItemTypeInput {
	name: string;
	code: string;
}

export interface CreateWorkItemInput {
	section_id: string;
	work_item_type_id: string;
	name?: string;
	priority?: Priority;
}

export interface CreatePropertyDefinitionInput {
	name: string;
	key?: string;
	data_type: PropertyDefinitionDto['data_type'];
	unit?: string;
	required?: boolean;
	options?: string[];
}

export interface SetPropertyValueInput {
	property_definition_id: string;
	value: string | number | boolean | null;
}

export const workItemTypeService = {
	list(): Promise<WorkItemTypeDto[]> {
		return api.get('/work-item-types');
	},
	create(input: CreateWorkItemTypeInput): Promise<WorkItemTypeDto> {
		return api.post('/work-item-types', input);
	}
};

export const workItemService = {
	listBySection(projectId: string, sectionId: string): Promise<WorkItemDto[]> {
		return api.get(`/projects/${projectId}/work-items?section_id=${sectionId}`);
	},

	/** Projenin tüm iş kalemleri (section_id verilmemiş istek). */
	listAll(projectId: string): Promise<WorkItemDto[]> {
		return api.get(`/projects/${projectId}/work-items`);
	},

	create(projectId: string, input: CreateWorkItemInput): Promise<WorkItemDto> {
		return api.post(`/projects/${projectId}/work-items`, input);
	},

	/** Üst bölümün altındaki tüm yaprak bölümlere aynı tipte iş kalemi ekler. */
	bulkCreate(
		projectId: string,
		parentSectionId: string,
		workItemTypeId: string
	): Promise<WorkItemDto[]> {
		return api.post(`/projects/${projectId}/work-items/bulk`, {
			parent_section_id: parentSectionId,
			work_item_type_id: workItemTypeId
		});
	},

	get(itemId: string): Promise<WorkItemDetailDto> {
		return api.get(`/work-items/${itemId}`);
	},

	update(itemId: string, patch: { name?: string; priority?: Priority }): Promise<WorkItemDto> {
		return api.patch(`/work-items/${itemId}`, patch);
	},

	remove(itemId: string): Promise<{ ok: boolean }> {
		return api.del(`/work-items/${itemId}`);
	},

	setValues(itemId: string, values: SetPropertyValueInput[]): Promise<{ ok: boolean }> {
		return api.put(`/work-items/${itemId}/property-values`, values);
	}
};

export const propertyDefinitionService = {
	list(): Promise<PropertyDefinitionDto[]> {
		return api.get('/property-definitions');
	},
	create(input: CreatePropertyDefinitionInput): Promise<PropertyDefinitionDto> {
		return api.post('/property-definitions', input);
	}
};

/** SELECT options JSON string'ini ayrıştırır. */
export function parseOptions(options?: string | null): string[] {
	if (!options) return [];
	try {
		return JSON.parse(options) as string[];
	} catch {
		return [];
	}
}
