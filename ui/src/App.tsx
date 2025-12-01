import { useState, useCallback, useEffect, useRef, type DragEvent, type MouseEvent } from 'react';
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
} from 'reactflow';
import 'reactflow/dist/style.css';
import init, { WasmWorkflow } from 'rust_flow_wasm';
import NodePalette from './components/NodePalette';
import PropertiesPanel from './components/PropertiesPanel';

const initialNodes: Node[] = [
  { id: '1', position: { x: 250, y: 5 }, data: { label: 'Start' }, type: 'input' },
];

let id = 0;
const getId = () => `dndnode_${id++}`;

function Flow() {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const [nodes, setNodes] = useState<Node[]>(initialNodes);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [reactFlowInstance, setReactFlowInstance] = useState<ReactFlowInstance | null>(null);
  const [wasmReady, setWasmReady] = useState(false);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);

  useEffect(() => {
    init().then(() => setWasmReady(true));
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

      const type = event.dataTransfer.getData('application/reactflow/type');
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

  const onNodeClick = useCallback((event: MouseEvent, node: Node) => {
    setSelectedNode(node);
  }, []);

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
  }, []);

  // Update selected node when nodes change
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

  const runWorkflow = async () => {
    if (!wasmReady) return;
    try {
      const workflow = new WasmWorkflow();

      // Build workflow from graph
      nodes.forEach(node => {
        if (node.data.label.includes('Start') || node.data.label.includes('Manual')) {
          workflow.add_manual_trigger(node.id);
        } else if (node.data.label.includes('Time')) {
          workflow.add_time_trigger(node.id, node.data.cron || "* * * * *");
        } else if (node.data.label.includes('Webhook')) {
          workflow.add_webhook_trigger(node.id, node.data.path || "/", node.data.method || "POST");
        } else if (node.data.label.includes('Log')) {
          workflow.add_console_output(node.id);
        } else if (node.data.label.includes('Set Data')) {
          // Demo data
          workflow.add_set_data(node.id, JSON.stringify({ message: "Hello from Wasm!" }));
        }
      });

      edges.forEach(edge => {
        workflow.add_connection(edge.source, edge.target);
      });

      await workflow.run();
      console.log("Workflow executed successfully!");
      alert("Workflow executed! Check console for output.");
    } catch (e) {
      console.error("Workflow execution failed:", e);
      alert(`Error: ${e}`);
    }
  };

  return (
    <div className="h-screen w-screen flex flex-col">
      <div className="p-4 bg-gray-800 text-white flex justify-between items-center border-b border-gray-700">
        <h1 className="text-xl font-bold">RustFlow UI</h1>
        <div className="flex gap-2 items-center">
          <span className={`px-2 py-1 rounded text-xs font-mono ${wasmReady ? 'bg-green-600' : 'bg-red-600'}`}>
            {wasmReady ? 'WASM READY' : 'LOADING WASM...'}
          </span>
          <button
            onClick={runWorkflow}
            disabled={!wasmReady}
            className="bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded disabled:opacity-50 font-semibold transition-colors"
          >
            Run Workflow
          </button>
        </div>
      </div>
      <div className="flex-1 flex">
        <NodePalette />
        <div className="flex-1 h-full" ref={reactFlowWrapper}>
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
          >
            <Background />
            <Controls />
          </ReactFlow>
        </div>
        <PropertiesPanel selectedNode={selectedNode} setNodes={setNodes} />
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
