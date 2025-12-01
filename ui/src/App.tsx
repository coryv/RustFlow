import { useState, useCallback, useEffect, useRef, type DragEvent, type MouseEvent, memo } from 'react';
import ReactFlow, {
  type Node,
  type Edge,
  Controls,
  Background,
  applyNodeChanges,
  applyEdgeChanges,
  type NodeChange,
  type EdgeChange,
  type Connection,
  addEdge,
  ReactFlowProvider,
  type ReactFlowInstance,
  Panel,
  Handle,
  Position,
} from 'reactflow';
import 'reactflow/dist/style.css';
import Sidebar from './components/Sidebar';
import PropertiesPanel from './components/PropertiesPanel';
import Toolbar from './components/Toolbar';
import { workflowToYaml, yamlToWorkflow } from './lib/workflow-utils';
import { Play, Clock, Globe, Database, Code, ArrowRightLeft, FileJson, Terminal, FileText, Cpu, BrainCircuit } from 'lucide-react';

// --- INLINED COMPONENTS TO FIX VITE MODULE LOADING ISSUES ---

const icons: Record<string, any> = {
  manual_trigger: Play,
  time_trigger: Clock,
  webhook_trigger: Globe,
  http_request: Globe,
  code: Code,
  set_data: Database,
  console_output: Terminal,
  router: ArrowRightLeft,
  notion_create_page: FileText,
  html_extract: Code,
  join: ArrowRightLeft,
  union: ArrowRightLeft,
  file_source: FileText,
  agent: Cpu,
};

const BaseNode = memo(({ data, type, selected }: any) => {
  const Icon = icons[type] || Globe;
  const triggerTypes = ['manual_trigger', 'time_trigger', 'webhook_trigger'];
  const isTrigger = triggerTypes.includes(type) || type.includes('trigger');

  return (
    <div className={`px-4 py-2 shadow-md rounded-md bg-gray-800 border-2 min-w-[150px] ${selected ? "border-blue-500" : "border-gray-700"}`}>
      {!isTrigger && (
        <Handle
          type="target"
          position={Position.Left}
          className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800"
        />
      )}

      <div className="flex items-center">
        <div className="rounded-full w-8 h-8 flex items-center justify-center bg-gray-700 text-gray-300 mr-3">
          <Icon size={16} />
        </div>
        <div className="ml-1">
          <div className="text-sm font-bold text-gray-200">{data.label}</div>
          {data.description && <div className="text-xs text-gray-500">{data.description}</div>}
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800"
      />
    </div>
  );
});

const AgentNode = memo(({ data, selected }: any) => {
  return (
    <div className={`shadow-md rounded-md bg-gray-900 border-2 min-w-[200px] ${selected ? "border-purple-500" : "border-gray-700"}`}>
      {/* Main Workflow Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="workflow-in"
        className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800 top-1/2"
      />

      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-800 flex items-center bg-gray-800 rounded-t-md">
        <div className="rounded-full w-8 h-8 flex items-center justify-center bg-purple-900/50 text-purple-400 mr-3">
          <Cpu size={18} />
        </div>
        <div>
          <div className="text-sm font-bold text-gray-200">AI Agent</div>
          <div className="text-xs text-gray-500">LLM Processor</div>
        </div>
      </div>

      {/* Body / Slots */}
      <div className="p-4 space-y-4">
        {/* Model Slot */}
        <div className="relative bg-gray-950/50 rounded p-2 border border-gray-800 border-dashed">
          <div className="text-xs text-gray-500 uppercase font-semibold mb-1">Model</div>
          <div className="text-xs text-gray-600 italic">Connect a model...</div>

          {/* Slot Handle */}
          <Handle
            type="target"
            position={Position.Left}
            id="model-slot"
            className="w-3 h-3 !bg-purple-500 !border-2 !border-gray-900 !rounded-none"
            style={{ left: -8, top: '50%' }}
          />
        </div>
      </div>

      {/* Main Workflow Output */}
      <Handle
        type="source"
        position={Position.Right}
        id="workflow-out"
        className="w-3 h-3 !bg-gray-400 !border-2 !border-gray-800 top-1/2"
      />
    </div>
  );
});

const ModelNode = memo(({ data, selected }: any) => {
  return (
    <div className={`px-3 py-2 shadow-md rounded-md bg-purple-900/20 border-2 min-w-[140px] ${selected ? "border-purple-400" : "border-purple-900/50"}`}>
      <div className="flex items-center">
        <div className="mr-2 text-purple-400">
          <BrainCircuit size={14} />
        </div>
        <div className="text-xs font-bold text-purple-200">{data.label}</div>
      </div>

      {/* Output Handle Only - Connects to Agent Slot */}
      <Handle
        type="source"
        position={Position.Right}
        className="w-3 h-3 !bg-purple-500 !border-2 !border-gray-900 !rounded-none"
      />
    </div>
  );
});

// -----------------------------------------------------------

const nodeTypes = {
  manual_trigger: BaseNode,
  time_trigger: BaseNode,
  webhook_trigger: BaseNode,
  http_request: BaseNode,
  code: BaseNode,
  set_data: BaseNode,
  console_output: BaseNode,
  router: BaseNode,
  notion_create_page: BaseNode,
  html_extract: BaseNode,
  join: BaseNode,
  union: BaseNode,
  file_source: BaseNode,
  agent: AgentNode,
  openai_model: ModelNode,
  gemini_model: ModelNode,
};

const initialNodes: Node[] = [
  { id: '1', position: { x: 250, y: 100 }, data: { label: 'Manual Trigger' }, type: 'manual_trigger' },
];

let id = 0;
const getId = () => `node_${id++}`;

function Flow() {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const [nodes, setNodes] = useState<Node[]>(initialNodes);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [reactFlowInstance, setReactFlowInstance] = useState<ReactFlowInstance | null>(null);
  const [serverReady, setServerReady] = useState(true); // Optimistic
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [nodeRegistry, setNodeRegistry] = useState<any[]>([]);

  useEffect(() => {
    // Check server health
    fetch('http://localhost:3000/health')
      .then(res => {
        if (res.ok) setServerReady(true);
        else setServerReady(false);
      })
      .catch(() => setServerReady(false));

    // Fetch node types
    fetch('http://localhost:3000/api/node-types')
      .then(res => res.json())
      .then(data => setNodeRegistry(data))
      .catch(console.error);
  }, []);

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => setNodes((nds) => applyNodeChanges(changes, nds)),
    [setNodes]
  );
  const onEdgesChange = useCallback(
    (changes: EdgeChange[]) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    [setEdges]
  );
  const onConnect = useCallback(
    (connection: Connection) => setEdges((eds) => addEdge(connection, eds)),
    [setEdges]
  );

  const onDragOver = useCallback((event: DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: DragEvent) => {
      event.preventDefault();

      if (!reactFlowWrapper.current || !reactFlowInstance) {
        return;
      }

      const type = event.dataTransfer.getData('application/reactflow');
      const label = event.dataTransfer.getData('application/reactflow/label');

      if (typeof type === 'undefined' || !type) {
        return;
      }

      const position = reactFlowInstance.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      const newNode: Node = {
        id: getId(),
        type,
        position,
        data: { label: `${label}` },
      };

      setNodes((nds) => nds.concat(newNode));
    },
    [reactFlowInstance]
  );

  const onNodeClick = useCallback((_: MouseEvent, node: Node) => {
    setSelectedNode(node);
  }, []);

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
  }, []);

  // Update selected node when nodes change (to keep properties panel in sync)
  useEffect(() => {
    if (selectedNode) {
      const node = nodes.find((n) => n.id === selectedNode.id);
      if (node) {
        setSelectedNode(node);
      } else {
        setSelectedNode(null);
      }
    }
  }, [nodes, selectedNode]);

  const handleRun = async () => {
    if (!serverReady) {
      alert("Server is not reachable. Please start the backend server.");
      return;
    }

    try {
      const yamlContent = workflowToYaml(nodes, edges);

      const response = await fetch('http://localhost:3000/api/run', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ workflow: yamlContent }),
      });

      const result = await response.json();

      if (result.status === 'success') {
        console.log("Workflow executed successfully!", result.logs);
        alert("Workflow executed successfully! Check console for logs.");
      } else {
        console.error("Workflow execution failed:", result.error);
        alert(`Error: ${result.error}`);
      }

    } catch (e) {
      console.error("Workflow execution failed:", e);
      alert(`Error: ${e}`);
    }
  };

  const handleExport = () => {
    const yamlContent = workflowToYaml(nodes, edges);
    const blob = new Blob([yamlContent], { type: 'text/yaml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'workflow.yaml';
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleImport = (content: string) => {
    const { nodes: newNodes, edges: newEdges } = yamlToWorkflow(content);
    setNodes(newNodes);
    setEdges(newEdges);
  };

  return (
    <div className="h-screen w-screen flex flex-col bg-gray-900 text-white overflow-hidden">
      <Toolbar
        onRun={handleRun}
        onExport={handleExport}
        onImport={handleImport}
        wasmReady={serverReady}
      />

      <div className="flex-1 flex overflow-hidden">
        <Sidebar nodeTypes={nodeRegistry} />

        <div className="flex-1 h-full relative" ref={reactFlowWrapper}>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onInit={setReactFlowInstance}
            onDrop={onDrop}
            onDragOver={onDragOver}
            onNodeClick={onNodeClick}
            onPaneClick={onPaneClick}
            nodeTypes={nodeTypes as any}
            fitView
            className="bg-gray-950"
          >
            <Background color="#333" gap={16} />
            <Controls className="bg-gray-800 border-gray-700 fill-gray-200" />
            <Panel position="bottom-right" className="bg-gray-800 p-2 rounded text-xs text-gray-400 border border-gray-700">
              RustFlow Builder v0.1
            </Panel>
          </ReactFlow>
        </div>

        <PropertiesPanel
          selectedNode={selectedNode}
          setNodes={setNodes}
          setSelectedNode={setSelectedNode}
          nodeTypes={nodeRegistry}
        />
      </div>
    </div>
  );
}

export default function App() {
  return (
    <ReactFlowProvider>
      <Flow />
    </ReactFlowProvider>
  );
}
