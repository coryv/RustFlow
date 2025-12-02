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

export interface ExecutionEvent {
    type: 'NodeStart' | 'NodeFinish' | 'EdgeData' | 'NodeError';
    node_id?: string;
    from?: string;
    to?: string;
    value?: any;
    error?: string;
}
