export interface NodeProperty {
    name: string;
    label: string;
    type: string; // text, number, select, json, code, boolean
    options?: string[];
    default?: string;
    required: boolean;
}

export interface NodeType {
    id: string;
    label: string;
    category: string;
    description?: string;
    properties: NodeProperty[];
}
