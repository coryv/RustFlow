import type { Node, Edge } from 'reactflow';
import yaml from 'js-yaml';

// Define RustFlow Schema Types
interface RustFlowNode {
    id: string;
    type: string;
    data?: any; // For 'set_data' and others using 'data' field
    config?: any; // For 'action' nodes
    // Add other fields as needed based on schema.rs
}

interface RustFlowEdge {
    from: string;
    to: string;
    from_port?: number;
    to_port?: number;
}

interface RustFlowWorkflow {
    name?: string;
    nodes: RustFlowNode[];
    edges: RustFlowEdge[];
}

export const workflowToYaml = (nodes: Node[], edges: Edge[]): string => {
    // 1. Flatten Graph: Merge "Model" nodes into "Agent" nodes
    const flattenedNodes = nodes.filter(n => !n.type?.includes('_model'));
    const modelNodes = nodes.filter(n => n.type?.includes('_model'));

    // Create a map of edges for quick lookup
    const edgeMap = edges.reduce((acc, edge) => {
        if (!acc[edge.target]) acc[edge.target] = [];
        acc[edge.target].push(edge);
        return acc;
    }, {} as Record<string, Edge[]>);

    const workflowNodes = flattenedNodes.map(node => {
        const nodeData = { ...node.data };

        // If Agent, look for connected Model
        if (node.type === 'agent') {
            const incomingEdges = edgeMap[node.id] || [];
            const modelEdge = incomingEdges.find(e => e.targetHandle === 'model-slot');

            if (modelEdge) {
                const modelNode = modelNodes.find(m => m.id === modelEdge.source);
                if (modelNode) {
                    // Merge model properties into agent
                    nodeData.model = modelNode.data.model;
                    nodeData.api_key = modelNode.data.api_key;
                    nodeData.provider = modelNode.type === 'openai_model' ? 'openai' : 'gemini';
                }
            }
        }

        return {
            id: node.id,
            type: node.type,
            ...nodeData
        };
    });

    // Filter out edges that were used for slots
    const workflowEdges = edges.filter(edge => {
        return edge.targetHandle !== 'model-slot' && edge.targetHandle !== 'tools-slot';
    }).map(edge => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
    }));

    const workflow = {
        nodes: workflowNodes,
        edges: workflowEdges,
    };

    return yaml.dump(workflow);
};

export const yamlToWorkflow = (yamlString: string): { nodes: Node[]; edges: Edge[] } => {
    try {
        const workflow = yaml.load(yamlString) as RustFlowWorkflow;

        if (!workflow || !workflow.nodes) {
            throw new Error("Invalid workflow YAML");
        }

        const nodes: Node[] = workflow.nodes.map((node, index) => {
            return {
                id: node.id,
                type: node.type,
                position: { x: 100 + (index * 150), y: 100 + (index * 50) }, // Basic auto-layout
                data: node.data || node.config || { label: node.id }, // Fallback label
            };
        });

        const edges: Edge[] = (workflow.edges || []).map((edge, i) => ({
            id: `e${i}`,
            source: edge.from,
            target: edge.to,
        }));

        return { nodes, edges };
    } catch (e) {
        console.error("Failed to parse YAML", e);
        return { nodes: [], edges: [] };
    }
};
